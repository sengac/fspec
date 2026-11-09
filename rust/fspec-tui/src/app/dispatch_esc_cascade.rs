//! App::dispatch routing for the Esc cascade (RPC-051 + RPC-095 + RPC-098).
//!
//! Feature: spec/features/keyboard-shortcut-cascade-parity.feature
//!          spec/features/agentview-multilineinput-parity.feature
//!          spec/features/agentview-esc-exit-confirmation-dialog.feature
//!
//! Routing decision tree for `Action::AgentEscPressed`:
//!
//! ```text
//! current_session() ─► Some(id)
//!                         │
//!                         ├─ session_status_for(id) ∈ {Running, Compacting}
//!                         │     → spawn `backend.interrupt(id)`
//!                         │       (Navigator stays at ViewMode::Agent)
//!                         │
//!                         ├─ input buffer non-empty (after trim) [RPC-095 L6]
//!                         │     → clear input, stay on AgentView
//!                         │
//!                         └─ otherwise [RPC-098 L7]
//!                               → request_close (TUI-124): fetch the close
//!                                 facts, then push the close dialog
//!                                 (guarded by Compositor::contains to avoid
//!                                 double-push)
//!
//! current_session() ─► None
//!                         → dispatch Action::BackToBoard
//! ```

use codelet_rpc_types::SessionStatus;

use crate::components::Action;

use super::state::App;

impl App {
    /// Route `Action::AgentEscPressed` per the cascade above.
    pub(crate) fn handle_agent_esc_pressed(&mut self) {
        let session = match self.agent_view_store.current_session().cloned() {
            Some(id) => id,
            None => {
                let _ = self.action_tx.send(Action::BackToBoard);
                return;
            }
        };
        let is_active = matches!(
            self.agent_view_store.session_status_for(&session).copied(),
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting),
        );
        if is_active {
            // L4/L5 — interrupt the live run, but stay on the
            // AgentView so the user sees the interrupt land.
            let backend = self.backend.clone();
            let handle = tokio::spawn(async move {
                let _ = backend.interrupt(session).await;
            });
            self.pending_tasks.push(handle);
            return;
        }
        // RPC-095 L6 — input buffer non-empty after trim:
        // clear the buffer and stay on AgentView.
        let input_has_text = !self.navigator.agent.input.value().trim().is_empty();
        if input_has_text {
            self.navigator.agent.input.reset();
            let _ = self
                .action_tx
                .send(Action::PendingInputChanged(String::new()));
            return;
        }
        // RPC-098 L7 / TUI-124 — gather what closing will do, then open
        // the close dialog (no-op while one is already open).
        self.request_close(session);
    }
}
