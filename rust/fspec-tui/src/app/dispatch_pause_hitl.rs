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
//!     pause slot and fire-and-forget the backend write. Errors are
//!     silently logged via `tracing` — no scrollback notice.
//!
//!   - `handle_pause_resumed`: fire-and-forget `backend.pause_resume`.
//!     RPC-406: NOT reachable from the inline pause prompt (Esc denies)
//!     — retained for other callers only.
//!
//!   - `handle_pause_prompt_enter`: reads the authoritative triple
//!     selection from the store, maps 0/1/2 →
//!     `ApprovalChoice::{Approve, ApproveSession, Deny}` and routes
//!     through `handle_pause_triple`.

use std::sync::Arc;

use codelet_rpc_types::{ApprovalChoice, HitlResponse, PauseState, SessionId};

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

impl App {
    /// RPC-053: react to `SessionStateChange { state: Paused }` by
    /// spawning parallel reads of `backend.get_pause_state` and
    /// `backend.get_hitl_request` and routing the first Some result
    /// back via `Action::PauseStateFetched` / `Action::HitlPromptFetched`.
    pub(crate) fn handle_pause_chunk(&mut self, session_id: SessionId) {
        // Sync unit-test fallback — do not spawn a task when no runtime.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            // Issue the reads in parallel so the dispatcher sees the
            // first Some result without paying serial latency.
            let (pause_res, hitl_res, exec_res) = tokio::join!(
                backend.get_pause_state(id.clone()),
                backend.get_hitl_request(id.clone()),
                backend.get_exec_stdin_request(id.clone()),
            );
            let pause = match pause_res {
                Ok(p) => p,
                Err(err) => {
                    tracing::debug!(
                        target: "rpc053",
                        session = &id.value,
                        error = %err,
                        "get_pause_state failed (silently dropped)",
                    );
                    None
                }
            };
            let hitl = match hitl_res {
                Ok(h) => h,
                Err(err) => {
                    tracing::debug!(
                        target: "rpc053",
                        session = &id.value,
                        error = %err,
                        "get_hitl_request failed (silently dropped)",
                    );
                    None
                }
            };
            // RPC-053 rule [13]: HITL wins on tie. RPC-411: store
            // slot instead of a modal push.
            if let Some(request) = hitl {
                let _ = action_tx.send(Action::HitlPromptFetched {
                    session_id: id,
                    request,
                });
                return;
            }
            if let Some(state) = pause {
                // RPC-406: store slot instead of a modal push.
                let _ = action_tx.send(Action::PauseStateFetched {
                    session_id: id.clone(),
                    state,
                });
            }
            // TOOL-022 P2: lowest-precedence overlay. Only surface the
            // exec-stdin request when neither HITL nor a pause occupies
            // the slot (the precedence chain is HITL > exec-stdin >
            // pause > composer). When the backend returns None the
            // probe clears the slot (exec session gone / no longer
            // quiet). The error is a transport failure — silently
            // dropped (logged), the overlay just doesn't appear this
            // tick.
            match exec_res {
                Ok(Some(request)) => {
                    let _ = action_tx.send(Action::ExecStdinPromptFetched {
                        agent_session: id,
                        request,
                    });
                }
                Ok(None) => {
                    let _ = action_tx.send(Action::ExecStdinDismissed {
                        agent_session: id,
                    });
                }
                Err(err) => {
                    tracing::debug!(
                        target: "tool022",
                        session = &id.value,
                        error = %err,
                        "get_exec_stdin_request failed (silently dropped)",
                    );
                }
            }
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

    /// RPC-406: fold a fetched PauseState into the per-session store
    /// slot. The AgentView paints the inline prompt from this slot on
    /// the next frame (only when the paused session is focused).
    pub(crate) fn handle_pause_state_fetched(&mut self, session_id: SessionId, state: PauseState) {
        self.agent_view_store.set_pause_state(session_id, state);
        self.should_render = true;
    }

    /// RPC-406: cycle the triple-prompt selection with wraparound.
    pub(crate) fn handle_pause_prompt_nav(&mut self, session_id: &SessionId, delta: i32) {
        self.agent_view_store
            .cycle_triple_pause_selection(session_id, delta);
        self.should_render = true;
    }

    /// RPC-406: Enter on the triple prompt — read the authoritative
    /// selection from the store, map onto an ApprovalChoice, and route
    /// through `handle_pause_triple` (which clears the slot).
    pub(crate) fn handle_pause_prompt_enter(&mut self, session_id: SessionId) {
        let choice = match self
            .agent_view_store
            .triple_pause_selection_for(&session_id)
        {
            0 => ApprovalChoice::Approve,
            1 => ApprovalChoice::ApproveSession,
            _ => ApprovalChoice::Deny,
        };
        self.handle_pause_triple(session_id, choice);
    }

    /// RPC-053: fire-and-forget `backend.pause_confirm(session, accept)`.
    /// RPC-406: clears the per-session pause slot so the inline prompt
    /// unmounts on the next frame. Errors are silently logged.
    pub(crate) fn handle_pause_confirmed(&mut self, session_id: SessionId, accept: bool) {
        self.agent_view_store.clear_pause_state(&session_id);
        self.should_render = true;
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.pause_confirm(id.clone(), accept).await {
                tracing::debug!(
                    target: "rpc053",
                    session = &id.value,
                    accept,
                    error = %err,
                    "pause_confirm failed (silently dropped)",
                );
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-053: fire-and-forget `backend.pause_triple(session, choice)`.
    /// RPC-406: clears the per-session pause slot (selection resets).
    pub(crate) fn handle_pause_triple(&mut self, session_id: SessionId, choice: ApprovalChoice) {
        self.agent_view_store.clear_pause_state(&session_id);
        self.should_render = true;
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.pause_triple(id.clone(), choice).await {
                tracing::debug!(
                    target: "rpc053",
                    session = &id.value,
                    error = %err,
                    "pause_triple failed (silently dropped)",
                );
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-053: fire-and-forget `backend.pause_resume(session)`.
    /// RPC-406: NOT reachable from the inline pause prompt — Esc
    /// denies. Retained for non-prompt callers only.
    pub(crate) fn handle_pause_resumed(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.pause_resume(id.clone()).await {
                tracing::debug!(
                    target: "rpc053",
                    session = &id.value,
                    error = %err,
                    "pause_resume failed (silently dropped)",
                );
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-053/RPC-411: fire-and-forget `backend.send_hitl_response`
    /// and clear the per-session HITL slot (submit AND cancel both
    /// route here — the response is always sent before the slot dies).
    pub(crate) fn handle_hitl_submitted(&mut self, session_id: SessionId, response: HitlResponse) {
        self.agent_view_store.clear_hitl_prompt(&session_id);
        self.should_render = true;
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            if let Err(err) = backend.send_hitl_response(id.clone(), response).await {
                tracing::debug!(
                    target: "rpc053",
                    session = &id.value,
                    error = %err,
                    "send_hitl_response failed (silently dropped)",
                );
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Route the RPC-053 / RPC-406 Action variants through their
    /// helpers. Called from the catch-all arm of `App::dispatch`.
    pub(crate) fn try_dispatch_pause_hitl(&mut self, action: &Action) -> bool {
        match action {
            Action::PauseChunkReceived(sid) => {
                self.handle_pause_chunk(sid.clone());
            }
            Action::PauseCleared(sid) => {
                self.handle_pause_cleared(sid.clone());
            }
            Action::PauseStateFetched { session_id, state } => {
                self.handle_pause_state_fetched(session_id.clone(), state.clone());
            }
            Action::PausePromptNav { session_id, delta } => {
                self.handle_pause_prompt_nav(session_id, *delta);
            }
            Action::PausePromptEnter { session_id } => {
                self.handle_pause_prompt_enter(session_id.clone());
            }

            Action::PauseConfirmed { session_id, accept } => {
                self.handle_pause_confirmed(session_id.clone(), *accept);
            }
            Action::PauseTriple { session_id, choice } => {
                self.handle_pause_triple(session_id.clone(), *choice);
            }
            Action::PauseResumed { session_id } => {
                self.handle_pause_resumed(session_id.clone());
            }
            Action::HitlSubmitted {
                session_id,
                response,
            } => {
                self.handle_hitl_submitted(session_id.clone(), response.clone());
            }
            _ => return self.try_dispatch_hitl_prompt(action),
        }
        true
    }
}
