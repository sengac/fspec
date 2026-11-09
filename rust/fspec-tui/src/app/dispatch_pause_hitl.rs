//! App::dispatch routing for RPC-053 Pause / HITL + RPC-406 inline
//! pause prompt.
//!
//! Features: spec/features/pause-and-hitl-dialogs.feature,
//! spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Hosts the impl App helpers invoked from `app/dispatch.rs` and
//! `app/dispatch_stream_chunks.rs` (chunk dispatcher):
//!
//!   - `handle_pause_chunk(session_id)`: fired from
//!     `handle_stream_chunk_state_updates` when
//!     `StreamChunk::SessionStateChange { state: Paused }` arrives. Spawns
//!     parallel `backend.get_pause_state` and `backend.get_hitl_request`
//!     reads and dispatches `Action::PauseStateFetched` on Some pause
//!     state (RPC-406: store slot, no modal) or `Action::HitlPromptFetched`
//!     on Some hitl request (HITL wins on tie). When both return None
//!     nothing happens (likely a stale Paused chunk from a now-resumed
//!     session).
//!
//!   - `handle_pause_cleared(session_id)`: fired from the chunk
//!     dispatcher on `Running` / `Idle`. Clears the RPC-406 per-session
//!     pause slot and the RPC-411 HITL slot so the UI does not
//!     strand a stale prompt after the agent loop resumes server-side.
//!
//!   - `handle_pause_confirmed` / `handle_pause_triple`: clear the
//!     pause slot and send the backend write in a task. BUG-395: a
//!     failure is sent as `PausedWriteFailed` (red scrollback notice,
//!     prompt read back); with no runtime the prompt stays and the
//!     failure is reported at once.
//!
//!   - `handle_pause_human_action`: send `backend.pause_human_action` from the
//!     human-action prompt (AMGR-023; BUG-395 as above).
//!
//!   - `handle_pause_prompt_enter`: reads the authoritative triple
//!     selection from the store, maps 0/1/2 →
//!     `ApprovalChoice::{Approve, ApproveSession, Deny}` and routes
//!     through `handle_pause_triple`.

use std::sync::Arc;

use codelet_rpc_types::{ApprovalChoice, HitlResponse, HumanActionReply, SessionId};

use tokio::sync::mpsc::UnboundedSender;

use crate::components::Action;
use crate::transport::FspecBackend;

use super::dispatch_exec_stdin_recovery::send_to_app;
use super::state::App;

impl App {
    /// RPC-053: react to `SessionStateChange { state: Paused }` by
    /// spawning parallel reads of `backend.get_pause_state` and
    /// `backend.get_hitl_request` and routing the first Some result
    /// back via `Action::PauseStateFetched` / `Action::HitlPromptFetched`.
    pub(crate) fn handle_pause_chunk(&mut self, session_id: SessionId) {
        // BUG-408: without a runtime no probe task can run; report it.
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::error!(
                session = &session_id.value,
                "no tokio runtime: pause/question/exec-stdin probe not sent"
            );
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let generation = self.agent_view_store.advance_exec_stdin_generation(&id);
        let handle = tokio::spawn(async move {
            // Issue the reads in parallel so the dispatcher sees the
            // first Some result without paying serial latency.
            let (pause_res, hitl_res, exec_res) = tokio::join!(
                backend.get_pause_state(id.clone()),
                backend.get_hitl_request(id.clone()),
                backend.get_exec_stdin_request(id.clone()),
            );
            // BUG-394: a failed read is reported, never folded into None.
            let pause = read_or_report(&action_tx, &id, "get_pause_state", pause_res);
            let hitl = read_or_report(&action_tx, &id, "get_hitl_request", hitl_res);
            // RPC-053 rule [13]: HITL wins on tie. RPC-411: store
            // slot instead of a modal push. BUG-394: when the HITL read
            // failed its precedence is unknown, so no pause prompt shows.
            match (hitl, pause) {
                (Some(Some(pending)), _) => {
                    send_to_app(
                        &action_tx,
                        Action::HitlPromptFetched {
                            session_id: id,
                            route: pending.route,
                            request: pending.payload,
                        },
                    );
                    return;
                }
                // RPC-406: store slot instead of a modal push.
                (Some(None), Some(Some(pending))) => send_to_app(
                    &action_tx,
                    Action::PauseStateFetched {
                        session_id: id.clone(),
                        route: pending.route,
                        state: pending.payload,
                    },
                ),
                _ => {}
            }
            // TOOL-022 P2: lowest-precedence overlay. Only surface the
            // exec-stdin request when neither HITL nor a pause occupies
            // the slot (the precedence chain is HITL > exec-stdin >
            // pause > composer). When the backend returns None the
            // probe clears the slot (exec session gone / no longer
            // quiet). BUG-364: a failed read reaches the app as
            // `ExecStdinProbeFailed`, which marks an open notice stale.
            let action = match exec_res {
                Ok(request) => Action::ExecStdinProbeFetched {
                    agent_session: id,
                    generation,
                    request,
                },
                Err(err) => Action::ExecStdinProbeFailed {
                    agent_session: id,
                    generation,
                    error: err.to_string(),
                },
            };
            send_to_app(&action_tx, action);
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-406/RPC-411: clear the per-session pause AND HITL slots
    /// when the agent loop resumes (Running or Idle) so the UI does
    /// not strand a stale inline prompt.
    pub(crate) fn handle_pause_cleared(&mut self, session_id: SessionId) {
        self.agent_view_store.clear_pause_state(&session_id);
        self.agent_view_store.clear_hitl_prompt(&session_id);
        self.should_render = true;
    }

    /// RPC-053: `backend.pause_confirm(session, request, accept)`. RPC-406:
    /// clears the per-session pause slot so the inline prompt unmounts on
    /// the next frame. BUG-395: a failed write is reported and the prompt
    /// read back ([`App::spawn_paused_write`]). AMGR-020: the reply names
    /// the pending request's id.
    pub(crate) fn handle_pause_confirmed(&mut self, session_id: SessionId, accept: bool) {
        let Some(request_id) = self.pause_request_id(&session_id, "pause_confirm") else {
            return;
        };
        self.agent_view_store.clear_pause_state(&session_id);
        self.should_render = true;
        self.spawn_paused_write(session_id, "pause_confirm", move |backend, id| async move {
            backend.pause_confirm(id, request_id, accept).await
        });
    }

    /// RPC-053: `backend.pause_triple(session, request, choice)`. RPC-406:
    /// clears the per-session pause slot (selection resets). BUG-395 and
    /// AMGR-020 as above.
    pub(crate) fn handle_pause_triple(&mut self, session_id: SessionId, choice: ApprovalChoice) {
        let Some(request_id) = self.pause_request_id(&session_id, "pause_triple") else {
            return;
        };
        self.agent_view_store.clear_pause_state(&session_id);
        self.should_render = true;
        self.spawn_paused_write(session_id, "pause_triple", move |backend, id| async move {
            backend.pause_triple(id, request_id, choice).await
        });
    }

    /// AMGR-023: `backend.pause_human_action(session, request, reply)`
    /// from the human-action prompt (Enter Done, Esc Cancel); clears the
    /// per-session pause slot. BUG-395 and AMGR-020 as above.
    pub(crate) fn handle_pause_human_action(
        &mut self,
        session_id: SessionId,
        reply: HumanActionReply,
    ) {
        let Some(request_id) = self.pause_request_id(&session_id, "pause_human_action") else {
            return;
        };
        self.agent_view_store.clear_pause_state(&session_id);
        self.should_render = true;
        self.spawn_paused_write(session_id, "pause_human_action", move |backend, id| async move {
            backend.pause_human_action(id, request_id, reply).await
        });
    }

    /// RPC-053/RPC-411: `backend.send_hitl_response` and clear the
    /// per-session HITL slot (submit AND cancel both route here — the
    /// response is always sent before the slot dies). BUG-395 and AMGR-020
    /// as above.
    pub(crate) fn handle_hitl_submitted(&mut self, session_id: SessionId, response: HitlResponse) {
        if !self.runtime_for_paused_write(&session_id, "send_hitl_response") {
            return;
        }
        let Some(request_id) = self
            .agent_view_store
            .hitl_prompt_for(&session_id)
            .map(|slot| slot.route.request_id.clone())
        else {
            self.report_paused_write_failed(
                &session_id,
                "send_hitl_response",
                "no pending question",
            );
            return;
        };
        self.agent_view_store.clear_hitl_prompt(&session_id);
        self.should_render = true;
        self.spawn_paused_write(
            session_id,
            "send_hitl_response",
            move |backend, id| async move { backend.send_hitl_response(id, request_id, response).await },
        );
    }

    /// AMGR-020: the id of the pending pause `write` answers; `None` (after
    /// reporting the failure) without a runtime or a pending pause.
    fn pause_request_id(&mut self, session_id: &SessionId, write: &str) -> Option<String> {
        if !self.runtime_for_paused_write(session_id, write) {
            return None;
        }
        let request_id = self
            .agent_view_store
            .pause_route_for(session_id)
            .map(|route| route.request_id.clone());
        if request_id.is_none() {
            self.report_paused_write_failed(session_id, write, "no pending approval");
        }
        request_id
    }

    /// BUG-395: `false` (after reporting the failure, prompt left in place)
    /// when there is no tokio runtime to send `write` on.
    fn runtime_for_paused_write(&mut self, session_id: &SessionId, write: &str) -> bool {
        if tokio::runtime::Handle::try_current().is_ok() {
            return true;
        }
        self.report_paused_write_failed(session_id, write, "no tokio runtime");
        false
    }

    /// BUG-395: run the backend `write` in a task; a failure reaches the
    /// app as `Action::PausedWriteFailed`, never dropped.
    fn spawn_paused_write<F, Fut>(&mut self, session_id: SessionId, write: &'static str, send: F)
    where
        F: FnOnce(Arc<dyn FspecBackend>, SessionId) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Err(err) = send(backend, session_id.clone()).await {
                send_to_app(
                    &action_tx,
                    Action::PausedWriteFailed {
                        session_id,
                        write: write.to_string(),
                        error: err.to_string(),
                    },
                );
            }
        });
        self.pending_tasks.push(handle);
    }
}

/// BUG-394: `Some(value)` for a successful Paused probe read; a failure
/// is sent to the app as `PausedReadFailed` and yields `None`.
fn read_or_report<T>(
    action_tx: &UnboundedSender<Action>,
    session: &SessionId,
    read: &str,
    result: anyhow::Result<T>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            send_to_app(
                action_tx,
                Action::PausedReadFailed {
                    session_id: session.clone(),
                    read: read.to_string(),
                    error: err.to_string(),
                },
            );
            None
        }
    }
}
