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
//! RPC-024 / BUG-549: on every session switch the composer text becomes
//! the outgoing (bound) session's `SessionContext.input_draft` and the
//! incoming session's saved draft is restored into the MultiLineInput
//! (`App::sync_composer_draft`, app/composer_draft.rs).
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
    /// session — focus it and switch the active view to AgentView
    /// immediately ([`Self::open_agent_view_on`], BUG-553).
    /// When `None` (BoardView Shift+Right on an unattached work unit)
    /// we mirror TS canonical `sessionGetNext()` semantics:
    ///
    ///   1. If `agent_view_store.first_open_session_id()` returns
    ///      `Some(sid)` — any open session is already in play — we
    ///      resume that session through the real focus path
    ///      ([`Self::open_agent_view_on`], BUG-553). NO dialog
    ///      appears. This is the RPC-097 reopen #2 fix: previously the dialog appeared
    ///      EVERY time the focused work unit had no attachment, even
    ///      when a session was already open from a previous round
    ///      trip (Shift+Right → Shift+Left → Shift+Right).
    ///
    ///   2. If no sessions are open, we mount the CreateSessionDialog
    ///      as an overlay on top of BoardView and leave
    ///      `navigator.active_view = ViewMode::Board`. The view
    ///      switch to AgentView only happens AFTER the user confirms
    ///      the dialog (see `handle_create_session_submitted` in
    ///      dispatch_create_session_dialog.rs). This is the RPC-097 reopen #1
    ///      contract, mirroring the canonical TS behavior in
    ///      `src/tui/views/BoardView.tsx` where `setViewMode('agent')`
    ///      is called from inside the dialog's `onConfirm` callback,
    ///      not at dialog open time.
    pub(crate) fn handle_open_agent_view(&mut self, target: Option<SessionId>) {
        match target {
            Some(sid) => self.open_agent_view_on(sid),
            None => {
                // RPC-097 reopen #2: probe the GLOBAL open-session
                // list before showing the dialog. This is the TS
                // canonical Shift+Right behaviour from
                // src/tui/utils/sessionNavigation.ts::navigateRight()
                // — if sessionGetNext returns Some, navigate to it;
                // only on None do we open the create dialog.
                if let Some(sid) = self.agent_view_store.first_open_session_id() {
                    self.open_agent_view_on(sid);
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

    /// BUG-553: open the agent view on exactly `session`, focused through
    /// the same path as every other route. An open tab is focused with
    /// [`Self::switch_to_session_index`] (BUG-549 draft swap, daemon
    /// active-session pointer, supervisors, exec-stdin probe); a session
    /// with no tab is attached like the resume list does (tab, hydration,
    /// transcript resume). The legacy navigation target is still recorded
    /// (RPC-012 / RPC-097) but the agent view shows the focused session.
    pub(crate) fn open_agent_view_on(&mut self, session: SessionId) {
        let open_index = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|context| context.id == session);
        match open_index {
            Some(index) => self.switch_to_session_index(index),
            None => self.handle_attach_to_session(session.clone()),
        }
        self.agent_view_store.set_navigation_target(Some(session));
        self.navigator.active_view = ViewMode::Agent;
        // BUG-197 R3/R4: leaving the board via a view flip drops the
        // board bar's ring focus so returning to the board paints NO
        // stale chip highlight (only the MENU-006 active-chip path may
        // highlight).
        self.board_store.menu_focus_to_columns();
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
                self.agent_view_store
                    .request_create_session_dialog_no_auto();
                self.handle_open_create_session_dialog(None);
            }
            NavTarget::Board => {
                // RPC-096: the composer keeps the focused session's draft
                // (recorded by the BUG-549 sync) so a later BoardView
                // round-trip restores the user's typing.
                self.sync_composer_draft();
                self.navigator.active_view = ViewMode::Board;
            }
        }
    }

    /// RPC-024 contract: focus `idx`; the BUG-549 sync saves the composer
    /// as the outgoing (bound) session's draft and restores the incoming
    /// draft; refresh supervisor badge (RPC-061 rule [9]).
    /// BUG-163: also used by `sync_mux_focus_to_session` (mux focus
    /// tracking) — `pub(crate)` so the mux dispatch module can reuse it.
    pub(crate) fn switch_to_session_index(&mut self, idx: usize) {
        // BUG-377: a missing incoming session is reported, never restored
        // as an empty draft; focus and the composer stay as they were.
        let incoming_session = match self.agent_view_store.focus_open_session(idx) {
            Ok(context) => context.id.clone(),
            Err(error) => {
                tracing::error!(%error, "session switch refused");
                return;
            }
        };
        // BUG-549: the outgoing draft is the BOUND session's (not the old
        // index's: an append may already have moved the index).
        self.sync_composer_draft();

        // TUI-113 R3: keep the daemon's active-session pointer in
        // sync with the focus (fire-and-forget).
        self.spawn_set_active_session(incoming_session.clone());
        self.spawn_load_supervisors(incoming_session.clone());
        // TOOL-022 P2: re-probe the exec-stdin overlay for the
        // incoming session on focus switch. The probe clears the
        // slot when the backend returns None (the exec session no
        // longer exists / is no longer quiet) and populates it
        // when a request is pending.
        self.probe_exec_stdin_for(incoming_session);
    }
}
