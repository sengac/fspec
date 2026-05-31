//! App::dispatch routing for RPC-024 multi-session cycling and RPC-096
//! end-of-list parity with the TS `useSessionNavigation` hook.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling pinned by `rpc013-source-shape.feature` +
//! `rpc024-source-shape.feature`.
//!
//! Hosts `handle_session_cycle`, the helper invoked from `App::dispatch`
//! when an `Action::SessionPrev` or `Action::SessionNext` arrives.
//!
//! RPC-024: on every session switch BEFORE mutating
//! `current_session_index`, snapshot the live `MultiLineInput` buffer
//! into the outgoing `SessionContext.input_draft`; AFTER mutating,
//! restore the incoming session's saved draft back into the
//! MultiLineInput.
//!
//! RPC-096: at the ends of the list, Shift+Right opens the Create
//! Session dialog and Shift+Left exits AgentView back to BoardView.

use codelet_rpc_types::SessionId;

use crate::store::NavTarget;
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// RPC-097: route `Action::OpenAgentView(target)`. When `target` is
    /// `Some(sid)` the user is jumping into an existing attached
    /// session — switch the active view to AgentView immediately.
    /// When `None` (BoardView Shift+Right on an unattached work unit)
    /// we mirror TS canonical `sessionGetNext()` semantics:
    ///
    ///   1. If `agent_view_store.first_open_session_id()` returns
    ///      `Some(sid)` — any open session is already in play — we
    ///      resume that session (set navigation_target + switch
    ///      active_view to Agent). NO dialog appears. This is the
    ///      RPC-097 reopen #2 fix: previously the dialog appeared
    ///      EVERY time the focused work unit had no attachment, even
    ///      when a session was already open from a previous round
    ///      trip (Shift+Right → Shift+Left → Shift+Right).
    ///
    ///   2. If no sessions are open, we mount the CreateSessionDialog
    ///      as an overlay on top of BoardView and leave
    ///      `navigator.active_view = ViewMode::Board`. The view
    ///      switch to AgentView only happens AFTER the user confirms
    ///      the dialog (see `handle_create_session_submitted` in
    ///      dispatch_rpc060.rs). This is the RPC-097 reopen #1
    ///      contract, mirroring the canonical TS behavior in
    ///      `src/tui/views/BoardView.tsx` where `setViewMode('agent')`
    ///      is called from inside the dialog's `onConfirm` callback,
    ///      not at dialog open time.
    pub(crate) fn handle_open_agent_view(&mut self, target: Option<SessionId>) {
        match target {
            Some(sid) => {
                self.agent_view_store
                    .set_navigation_target(Some(sid));
                self.navigator.active_view = ViewMode::Agent;
            }
            None => {
                // RPC-097 reopen #2: probe the GLOBAL open-session
                // list before showing the dialog. This is the TS
                // canonical Shift+Right behaviour from
                // src/tui/utils/sessionNavigation.ts::navigateRight()
                // — if sessionGetNext returns Some, navigate to it;
                // only on None do we open the create dialog.
                if let Some(sid) = self.agent_view_store.first_open_session_id() {
                    self.agent_view_store.set_navigation_target(Some(sid));
                    self.navigator.active_view = ViewMode::Agent;
                } else {
                    // No open sessions: preserve RPC-097 reopen #1
                    // contract — dialog overlays BoardView and the
                    // user remains on the board until they confirm.
                    self.agent_view_store
                        .request_create_session_dialog_no_auto();
                    self.handle_open_create_session_dialog(None);
                }
            }
        }
    }

    /// Resolve a Shift+Left/Right keypress (`delta == -1` for prev,
    /// `delta == 1` for next) against the multi-session navigation
    /// model, then either switch sessions (RPC-024 draft round-trip),
    /// open the Create Session dialog (RPC-096 off-right), or exit to
    /// BoardView (RPC-096 off-left).
    pub(crate) fn handle_session_cycle(&mut self, delta: isize) {
        let target = if delta < 0 {
            self.agent_view_store.navigate_prev()
        } else {
            self.agent_view_store.navigate_next()
        };
        match target {
            NavTarget::Session(idx) => self.switch_to_session_index(idx),
            NavTarget::CreateDialog => {
                // RPC-097: preserve the typed draft (the user has not
                // left the current session — they are just summoning a
                // modal) AND actually mount the dialog onto the
                // Compositor. Earlier the only side-effect was setting
                // `agent_view_store.show_create_session_dialog`, which
                // no render-pipeline subscriber consumed, so the
                // dialog never appeared. Delegate to the proven
                // RPC-060 helper which:
                //   * pushes CreateSessionDialog at Priority::Foreground,
                //   * reads the current session's WorkUnitContext for
                //     context-aware title rendering,
                //   * wires action_tx for CreateSessionSubmitted /
                //     CreateSessionCancelled, and
                //   * is idempotent on CREATE_SESSION_DIALOG_ID.
                self.agent_view_store.request_create_session_dialog_no_auto();
                self.handle_open_create_session_dialog(None);
            }
            NavTarget::Board => {
                // RPC-096: snapshot the outgoing draft so a later
                // BoardView round-trip restores the user's typing.
                let outgoing_idx = self.agent_view_store.current_session_index();
                let outgoing_draft = self.navigator.agent.input.value();
                self.agent_view_store
                    .set_input_draft(outgoing_idx, outgoing_draft);
                self.navigator.active_view = ViewMode::Board;
            }
        }
    }

    /// RPC-024 contract: snapshot outgoing draft, focus `idx`, restore
    /// incoming draft, refresh supervisor badge (RPC-061 rule [9]).
    fn switch_to_session_index(&mut self, idx: usize) {
        let outgoing_idx = self.agent_view_store.current_session_index();
        let outgoing_draft = self.navigator.agent.input.value();
        self.agent_view_store
            .set_input_draft(outgoing_idx, outgoing_draft);

        self.agent_view_store.focus_session_index(idx);

        let incoming_draft = self
            .agent_view_store
            .current_session_context()
            .map(|c| c.input_draft.clone())
            .unwrap_or_default();
        self.navigator.agent.input.set_value(&incoming_draft);

        if let Some(incoming_session) = self
            .agent_view_store
            .current_session_context()
            .map(|c| c.id.clone())
        {
            self.spawn_load_supervisors(incoming_session);
        }
    }
}
