//! RPC-061 — App::dispatch routing for supervisor / subordinate links.
//!
//! Feature: spec/features/rpc061-supervisor-links.feature
//!
//! Factored into its own file (mirroring the dispatch_rpc05X / 060
//! pattern) so `app/dispatch.rs` stays under the 300-LoC ceiling pinned
//! by `rpc024-source-shape.feature`.
//!
//! Three responsibilities:
//!
//! 1. `handle_supervisors_loaded` — write a fresh
//!    `backend.get_supervisors(session_id)` snapshot into
//!    `AgentViewStore`.
//! 2. `handle_send_to_subordinate` — spawn
//!    `backend.receive_incoming_message(subordinate_id, message)`;
//!    on `Err` emit `Action::EmitSessionNotice` against the
//!    originating supervisor session.
//! 3. `spawn_load_supervisors` — fire-and-forget
//!    `backend.get_supervisors(session_id)` that re-enters the action
//!    bus via `Action::SupervisorsLoaded`. Invoked by `Action::SessionCreated`.

use codelet_rpc_types::{IncomingMessageInput, SessionId};
use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-061: write the fresh supervisor snapshot into the
    /// AgentViewStore so the SessionHeader badge renders on the next
    /// frame.
    pub(crate) fn handle_supervisors_loaded(
        &mut self,
        session_id: SessionId,
        supervisors: Vec<SessionId>,
    ) {
        self.agent_view_store
            .set_supervisors(session_id, supervisors);
    }

    /// RPC-061: forward `message` onto `subordinate_id` via
    /// `backend.receive_incoming_message`. On `Err` emit a session
    /// notice against the originating supervisor session.
    pub(crate) fn handle_send_to_subordinate(
        &mut self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let originating_session = self.agent_view_store.current_session().cloned();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if let Err(e) = backend
                .receive_incoming_message(subordinate_id, message)
                .await
            {
                if let Some(sid) = originating_session {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        sid,
                        format!("[error] send to subordinate: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-061: spawn an async `backend.get_supervisors(session_id)`
    /// round-trip whose result is folded back into the action bus as
    /// `Action::SupervisorsLoaded(session_id, supervisors)`. Invoked
    /// from `Action::SessionCreated`, `Action::AttachToSession`, and
    /// session-cycle (`SessionPrev` / `SessionNext`) per rule [9].
    pub(crate) fn spawn_load_supervisors(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if let Ok(supervisors) = backend.get_supervisors(session_id.clone()).await {
                let _ = action_tx.send(Action::SupervisorsLoaded(session_id, supervisors));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Route RPC-061 Action variants through their helpers. Called from
    /// the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_rpc061(&mut self, action: &Action) -> bool {
        match action {
            Action::SupervisorsLoaded(session_id, supervisors) => {
                self.handle_supervisors_loaded(session_id.clone(), supervisors.clone());
            }
            Action::SendToSubordinate {
                subordinate_id,
                message,
            } => {
                self.handle_send_to_subordinate(subordinate_id.clone(), message.clone());
            }
            _ => return false,
        }
        true
    }
}
