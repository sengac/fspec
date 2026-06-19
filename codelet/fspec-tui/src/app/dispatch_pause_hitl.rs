//! App::dispatch routing for RPC-053 — Pause / HITL UI.
//!
//! Feature: spec/features/pause-and-hitl-dialogs.feature
//!
//! Hosts the impl App helpers invoked from `app/dispatch.rs` and
//! `app/dispatch_stream_chunks.rs` (chunk dispatcher):
//!
//!   - `handle_pause_chunk(session_id)`: fired from
//!     `handle_stream_chunk_state_updates` when
//!     `StreamChunk::SessionStateChange { state: Paused }` arrives. Spawns
//!     parallel `backend.get_pause_state` and `backend.get_hitl_request`
//!     reads and dispatches `Action::OpenPauseDialog` on Some pause state
//!     or `Action::OpenHitlDialog` on Some hitl request (HITL wins on
//!     tie). When both return None nothing happens (likely a stale
//!     Paused chunk from a now-resumed session).
//!
//!   - `handle_pause_cleared(session_id)`: fired from the chunk
//!     dispatcher on `Running` / `Idle`. Pops any mounted PauseDialog
//!     or HitlDialog so the UI does not strand a stale dialog after the
//!     agent loop resumes server-side.
//!
//!   - `handle_open_pause_dialog(session, state)` / `handle_open_hitl_dialog(session, request)`:
//!     idempotent compositor push (no-op on dialog-id collision).
//!
//!   - `handle_pause_confirmed` / `handle_pause_triple` /
//!     `handle_pause_resumed` / `handle_hitl_submitted`: fire-and-forget
//!     backend writes. The dialog has already removed itself from the
//!     Compositor via its EventResult::Consumed callback by the time
//!     these helpers run — they're invoked from the Action match arm in
//!     `app/dispatch.rs` AFTER the dialog's callback executed (the
//!     compositor `update(action)` fan-out is also called post-dispatch).
//!     Errors are silently logged via `tracing` — no scrollback notice.

use std::sync::Arc;

use codelet_rpc_types::{ApprovalChoice, HitlRequest, HitlResponse, PauseState, SessionId};

use crate::components::{
    hitl_dialog::{HitlDialog, HITL_DIALOG_ID},
    pause_dialog::{PauseDialog, PAUSE_DIALOG_ID},
    Action,
};
use crate::transport::FspecBackend;

use super::state::App;

impl App {
    /// RPC-053: react to `SessionStateChange { state: Paused }` by
    /// spawning parallel reads of `backend.get_pause_state` and
    /// `backend.get_hitl_request` and routing the first Some result
    /// back via `Action::OpenPauseDialog` / `Action::OpenHitlDialog`.
    pub(crate) fn handle_pause_chunk(&mut self, session_id: SessionId) {
        // Sync unit-test fallback — do not spawn a task when no runtime.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let id = session_id;
        let handle = tokio::spawn(async move {
            // Issue both reads in parallel so the dispatcher sees the
            // first Some result without paying serial latency.
            let (pause_res, hitl_res) = tokio::join!(
                backend.get_pause_state(id.clone()),
                backend.get_hitl_request(id.clone()),
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
            // RPC-053 rule [13]: HITL wins on tie.
            if let Some(request) = hitl {
                let _ = action_tx.send(Action::OpenHitlDialog {
                    session_id: id,
                    request,
                });
                return;
            }
            if let Some(state) = pause {
                let _ = action_tx.send(Action::OpenPauseDialog {
                    session_id: id,
                    state,
                });
            }
            // Both None → silent no-op.
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-053: pop any mounted PauseDialog / HitlDialog for this
    /// session when the agent loop resumes (Running or Idle).
    pub(crate) fn handle_pause_cleared(&mut self, _session_id: SessionId) {
        // Both dialog ids are singletons per App (one pause/hitl flow
        // visible at a time). The matched dialog already owns its own
        // session id internally; we simply pop both ids and let
        // Compositor::remove be a no-op when the id is not mounted.
        let _ = self.compositor.remove(PAUSE_DIALOG_ID);
        let _ = self.compositor.remove(HITL_DIALOG_ID);
    }

    /// RPC-053: idempotent compositor push for the PauseDialog.
    pub(crate) fn handle_open_pause_dialog(&mut self, session_id: SessionId, state: PauseState) {
        if self.compositor.contains(PAUSE_DIALOG_ID) {
            return;
        }
        let dialog = PauseDialog::new(session_id, state).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
        self.should_render = true;
    }

    /// RPC-053: idempotent compositor push for the HitlDialog.
    pub(crate) fn handle_open_hitl_dialog(&mut self, session_id: SessionId, request: HitlRequest) {
        if self.compositor.contains(HITL_DIALOG_ID) {
            return;
        }
        let dialog = HitlDialog::new(session_id, request).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
        self.should_render = true;
    }

    /// RPC-053: fire-and-forget `backend.pause_confirm(session, accept)`.
    /// The dialog has already removed itself via its callback by the
    /// time this dispatch arm runs. Errors are silently logged.
    pub(crate) fn handle_pause_confirmed(&mut self, session_id: SessionId, accept: bool) {
        // Ensure the dialog is gone even if a stale push survived.
        let _ = self.compositor.remove(PAUSE_DIALOG_ID);
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
    pub(crate) fn handle_pause_triple(&mut self, session_id: SessionId, choice: ApprovalChoice) {
        let _ = self.compositor.remove(PAUSE_DIALOG_ID);
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

    /// RPC-053: fire-and-forget `backend.pause_resume(session)` from
    /// the user pressing Esc on the PauseDialog.
    pub(crate) fn handle_pause_resumed(&mut self, session_id: SessionId) {
        let _ = self.compositor.remove(PAUSE_DIALOG_ID);
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

    /// RPC-053: fire-and-forget `backend.send_hitl_response`.
    pub(crate) fn handle_hitl_submitted(&mut self, session_id: SessionId, response: HitlResponse) {
        let _ = self.compositor.remove(HITL_DIALOG_ID);
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

    /// Route the RPC-053 Action variants through their helpers.
    /// Called from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_pause_hitl(&mut self, action: &Action) -> bool {
        match action {
            Action::PauseChunkReceived(sid) => {
                self.handle_pause_chunk(sid.clone());
            }
            Action::PauseCleared(sid) => {
                self.handle_pause_cleared(sid.clone());
            }
            Action::OpenPauseDialog { session_id, state } => {
                self.handle_open_pause_dialog(session_id.clone(), state.clone());
            }
            Action::OpenHitlDialog {
                session_id,
                request,
            } => {
                self.handle_open_hitl_dialog(session_id.clone(), request.clone());
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
            _ => return false,
        }
        true
    }
}
