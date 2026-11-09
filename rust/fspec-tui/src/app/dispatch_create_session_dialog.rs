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
use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-060: route an `Action::OpenCreateSessionDialog`. Pushes a
    /// fresh [`CreateSessionDialog`] onto the Compositor at
    /// Priority::Foreground, preselecting the option indicated by
    /// `preselect`. Generic creation always has absent story intent.
    /// Idempotent on dialog id — a second open is a no-op.
    pub(crate) fn handle_open_create_session_dialog(
        &mut self,
        preselect: Option<CreateSessionOption>,
    ) {
        if self.compositor.contains(CREATE_SESSION_DIALOG_ID) {
            return;
        }
        self.open_creation_dialog(preselect, None);
    }

    pub(crate) fn open_creation_dialog(
        &mut self,
        preselect: Option<CreateSessionOption>,
        context: Option<codelet_rpc_types::WorkUnitContext>,
    ) {
        if self.compositor.contains(CREATE_SESSION_DIALOG_ID) {
            return;
        }
        self.pending_creation_context.clone_from(&context);
        let dialog =
            CreateSessionDialog::new(preselect, context).with_action_tx(self.action_tx.clone());
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
        self.submit_session_creation(isolated, None);
    }

    /// TUI-114: mount the create dialog in New Bot mode (idempotent on
    /// the dialog id, like New Agent). Bots are never bound to a card.
    pub(crate) fn handle_open_new_bot_dialog(&mut self) {
        if self.compositor.contains(CREATE_SESSION_DIALOG_ID) {
            return;
        }
        self.pending_creation_context = None;
        let dialog = CreateSessionDialog::new_bot(None).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// Shared New Agent / New Bot submit: `role` is the session role
    /// (the orchestrator role for a bot, `None` for an agent).
    fn submit_session_creation(&mut self, isolated: bool, role: Option<String>) {
        let card = self
            .pending_creation_context
            .take()
            .map(|context| context.id);
        if self.navigator.active_view == crate::views::ViewMode::Board {
            self.navigator.active_view = crate::views::ViewMode::Agent;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let originating_session = self.agent_view_store.current_session().cloned();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let workspace = self
            .board_store
            .selected_other_workspace()
            .map(str::to_owned);
        let handle: JoinHandle<()> =
            tokio::spawn(super::session_creation_tasks::run_session_creation(
                backend,
                super::session_creation_tasks::CreationRequest {
                    isolated,
                    role,
                    workspace,
                    card,
                },
                None,
                originating_session,
                action_tx,
            ));
        self.pending_tasks.push(handle);
    }

    /// Fold a freshly-created (non-empty) session into the AgentViewStore and
    /// wire its chrome/attachments. Extracted from `App::dispatch` so the
    /// orchestrator file stays under the 300-LoC ceiling (PROV-101 FIX 3).
    pub(crate) fn handle_session_created(&mut self, session: codelet_rpc_types::SessionId) {
        self.initialize_session(session, super::session_arrival::SessionFocus::Take);
    }

    /// TUI-125: `focus` says whether the session takes focus (the TUI's own
    /// creation answer, the startup replay) or keeps the current one (a
    /// session the daemon announced, see `session_arrival.rs`).
    pub(crate) fn initialize_session(
        &mut self,
        session: codelet_rpc_types::SessionId,
        focus: super::session_arrival::SessionFocus,
    ) {
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
            // TUI-125: the creation answer for a tab the daemon's
            // announcement opened unfocused focuses it (once).
            if focus == super::session_arrival::SessionFocus::Take {
                self.focus_announced_creation(&session);
            }
            return;
        }
        if focus == super::session_arrival::SessionFocus::Keep
            && self.agent_view_store.current_session().is_some()
        {
            self.open_session_unfocused(session.clone());
            self.wire_opened_session(session);
            return;
        }
        self.agent_view_store
            .append_session(crate::store::SessionContext::new(session.clone()));
        self.rebuild_focus_occupancy();
        // MUX-002: re-sync the agent window to the grown session list;
        // if a mux Shift+Right new-agent prompt just confirmed, advance
        // the window so the new (last) session lands in the last agent
        // slot and focus moves there.
        self.mux_sync_window();
        self.navigator.mux.note_session_created();
        // BUG-163: a mux Shift+Right rotation moved focus to the new
        // agent pane — follow it with the store's current session
        // (RPC-024 draft round-trip). No-op outside mux mode.
        self.sync_mux_focus_to_session();
        let _ = self.active_session_tx.send(Some(session.clone()));
        // TUI-113 R3: publish the newly-created (and now focused) session
        // to the daemon's active-session pointer (fire-and-forget). This
        // fires for every focused creation — direct SessionCreated fold,
        // routed bootstrap/lazy creations (TUI-125: a session the daemon
        // announced does not take focus and does not move the pointer).
        self.spawn_set_active_session(session.clone());
        self.wire_opened_session(session);
    }

    /// The per-session fetches every newly opened tab gets, focused or not.
    fn wire_opened_session(&mut self, session: codelet_rpc_types::SessionId) {
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
            Action::OpenNewBotDialog => self.handle_open_new_bot_dialog(),
            Action::CreateBotSubmitted { isolated } => self.submit_session_creation(
                *isolated,
                Some(codelet_sessions::bot::ORCHESTRATOR_ROLE.to_string()),
            ),
            Action::BotSessionCreated(session) => {
                self.agent_view_store.mark_bot_session(session.clone());
            }
            Action::CreateSessionCancelled => {
                self.pending_creation_context = None;
                self.navigator.mux.cancel_new_agent_request();
            }
            _ => return false,
        }
        true
    }
}
