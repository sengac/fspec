//! RPC-060 — App::dispatch routing for the CreateSessionDialog +
//! isolated-session creation actions.
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//!
//! Factored into its own file (mirroring the dispatch_rpc05X pattern)
//! to keep `app/dispatch.rs` under the 300-LoC ceiling. The /isolation
//! slash command pick dispatches `Action::OpenCreateSessionDialog`,
//! the dialog itself emits `CreateSessionSubmitted` / `CreateSessionCancelled`,
//! and the dispatch helpers below fan out:
//!
//!  * `OpenCreateSessionDialog` → push CreateSessionDialog onto the
//!    compositor at Priority::Foreground (idempotent on id).
//!  * `CreateSessionSubmitted { isolated: true }` → spawn
//!    `backend.create_isolated_session(None)`.
//!  * `CreateSessionSubmitted { isolated: false }` → spawn
//!    `backend.create_session(None)`.
//!  * `CreateSessionCancelled` → silent no-op (dialog already popped
//!    itself via its return callback).

use tokio::task::JoinHandle;

use crate::components::Action;
use crate::components::create_session_dialog::{
    CreateSessionDialog, CreateSessionOption, CREATE_SESSION_DIALOG_ID,
};

use super::state::App;

impl App {
    /// RPC-060: route an `Action::OpenCreateSessionDialog`. Pushes a
    /// fresh [`CreateSessionDialog`] onto the Compositor at
    /// Priority::Foreground, preselecting the option indicated by
    /// `preselect`. The dialog reads the current session's
    /// `WorkUnitContext` (if any) so the title renders the
    /// context-aware `"Work on <id>?"` string. Idempotent on dialog
    /// id — a second open is a no-op.
    pub(crate) fn handle_open_create_session_dialog(
        &mut self,
        preselect: Option<CreateSessionOption>,
    ) {
        if self.compositor.contains(CREATE_SESSION_DIALOG_ID) {
            return;
        }
        let work_unit_context = self
            .agent_view_store
            .current_session()
            .cloned()
            .and_then(|sid| {
                self.agent_view_store
                    .work_unit_context_for(&sid)
                    .cloned()
            });
        let dialog = CreateSessionDialog::new(preselect, work_unit_context)
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// RPC-060: route an `Action::CreateSessionSubmitted { isolated }`.
    /// Spawns `backend.create_isolated_session(None)` (isolated=true)
    /// or `backend.create_session(None)` (isolated=false). On Ok the
    /// returned SessionId is wrapped in `Action::SessionCreated` and
    /// fed back through the action bus. On Err the focused session
    /// (if any) receives a `[error] create [isolated ]session: <e>`
    /// notice via `Action::EmitSessionNotice`.
    ///
    /// RPC-097: if the user confirmed the dialog while still on
    /// BoardView (the dialog was overlaying the board because they
    /// pressed Shift+Right on an unattached work unit), flip the
    /// active view to AgentView NOW — mirroring the TS canonical
    /// flow in `src/tui/views/BoardView.tsx` where `setViewMode('agent')`
    /// is called from inside the dialog's `onConfirm` callback.
    pub(crate) fn handle_create_session_submitted(&mut self, isolated: bool) {
        if self.navigator.active_view == crate::views::ViewMode::Board {
            self.navigator.active_view = crate::views::ViewMode::Agent;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let originating_session = self.agent_view_store.current_session().cloned();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if isolated {
                match backend.create_isolated_session(None).await {
                    Ok(info) => {
                        let _ = action_tx.send(Action::SessionCreated(info.session_id));
                    }
                    Err(e) => {
                        if let Some(sid) = originating_session {
                            let _ = action_tx.send(Action::EmitSessionNotice(
                                sid,
                                format!("[error] create isolated session: {e}"),
                            ));
                        }
                    }
                }
            } else {
                match backend.create_session(None).await {
                    Ok(session_id) => {
                        let _ = action_tx.send(Action::SessionCreated(session_id));
                    }
                    Err(e) => {
                        if let Some(sid) = originating_session {
                            let _ = action_tx.send(Action::EmitSessionNotice(
                                sid,
                                format!("[error] create session: {e}"),
                            ));
                        }
                    }
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Route RPC-060 Action variants through their helpers. Called from
    /// the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_rpc060(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenCreateSessionDialog { preselect } => {
                self.handle_open_create_session_dialog(*preselect);
            }
            Action::CreateSessionSubmitted { isolated } => {
                self.handle_create_session_submitted(*isolated);
            }
            Action::CreateSessionCancelled => {
                // Silent no-op — dialog popped via callback.
            }
            _ => return false,
        }
        true
    }
}
