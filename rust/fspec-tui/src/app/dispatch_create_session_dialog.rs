//! App::dispatch routing for the CreateSessionDialog +
//! isolated-session creation actions. Introduced: RPC-060.
//!
//! Feature: spec/features/rpc060-isolated-session-dialog.feature
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. The /isolation
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

use crate::components::create_session_dialog::{
    CreateSessionDialog, CreateSessionOption, CREATE_SESSION_DIALOG_ID,
};
use crate::components::error_dialog::{ErrorDialog, ERROR_DIALOG_ID};
use crate::components::Action;

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
            .and_then(|sid| self.agent_view_store.work_unit_context_for(&sid).cloned());
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
                        // PROV-101 FIX 1: an empty id is a decline (no default
                        // model). Map it to the explicit decline action — never
                        // append an empty-id session.
                        let _ = action_tx.send(
                            crate::app::session_creation::post_create_session_action(session_id),
                        );
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

    /// Fold a freshly-created (non-empty) session into the AgentViewStore and
    /// wire its chrome/attachments. Extracted from `App::dispatch` so the
    /// orchestrator file stays under the 300-LoC ceiling (PROV-101 FIX 3).
    pub(crate) fn handle_session_created(&mut self, session: codelet_rpc_types::SessionId) {
        // RPC-385: idempotent guard. The session-created broadcast folds into
        // Action::SessionCreated for EVERY creation path, so a tab the TUI
        // already opened (create-session dialog, isolated dialog, enter-work-
        // unit) must not be re-appended, must not steal focus, and must not
        // re-fire the chrome/supervisor/pending-input fetches. A spawned
        // subordinate (no pre-existing tab) falls through and is wired once.
        //
        // This early return is the SIDE-EFFECT-SUPPRESSION optimization: it
        // exists to avoid re-firing the chrome/supervisor/pending-input work
        // below for an already-open tab. The authoritative store-level dedup
        // invariant lives in `AgentViewStore::append_session`
        // (store/agent_view.rs); append remains a no-op even if this guard is
        // bypassed.
        if self
            .agent_view_store
            .session_context_for(&session)
            .is_some()
        {
            return;
        }
        self.agent_view_store
            .append_session(crate::store::SessionContext::new(session.clone()));
        let _ = self.active_session_tx.send(Some(session.clone()));
        if let Some(id) = self
            .agent_view_store
            .current_work_unit_id()
            .map(std::string::ToString::to_string)
        {
            let _ = self
                .action_tx
                .send(Action::AttachSession(id.clone(), session.clone()));
            // RPC-050: late-binding attach for the lazy-session path.
            let _ = self.action_tx.send(Action::AttachWorkUnitToSession(id));
        }
        self.refresh_session_chrome(session.clone());
        self.spawn_hydrate_pending_input(session.clone()); // RPC-052
        self.spawn_load_supervisors(session.clone()); // RPC-061

        // RPC-430: propagate pre-session debug state to the new session
        // if the user toggled /debug before any session existed.
        if self.pre_session_debug_enabled {
            let backend = self.backend.clone();
            let action_tx = self.action_tx.clone();
            let sid = session;
            let handle = tokio::spawn(async move {
                let _ = backend.set_debug_enabled(sid.clone(), true).await;
                let _ = action_tx.send(Action::DebugEnabledLoaded(sid, true));
            });
            self.pending_tasks.push(handle);
        }
    }

    /// PROV-101 FIX 1: surface a declined `create_session` (empty SessionId,
    /// i.e. no default model is set) as a Priority::Critical [`ErrorDialog`] so
    /// the user is alerted instead of an empty-id session being silently
    /// appended. Idempotent: a second decline while the dialog is open is a
    /// no-op.
    pub(crate) fn handle_session_creation_declined(&mut self) {
        if !self.compositor.contains(ERROR_DIALOG_ID) {
            let dialog = ErrorDialog::new(
                "Session creation declined: no default model is set. \
                 Select a model first (/model)."
                    .to_string(),
            );
            self.compositor.push(Box::new(dialog));
        }
    }

    /// Route RPC-060 Action variants through their helpers. Called from
    /// the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_create_session_dialog(&mut self, action: &Action) -> bool {
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
