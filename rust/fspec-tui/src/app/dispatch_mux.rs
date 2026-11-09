//! MUX-001 — `App::dispatch` arms for the mux `Action` variants.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Store-level work (config application, persistence, error notices)
//! lives here; the Navigator's `apply_action` handles the view flip +
//! focus. `App::dispatch` runs this arm and then calls
//! `navigator.apply_action`, so the two halves compose per action.

use crate::components::menu_bar::items::MenuAction;
use crate::components::notification_dialog::{NotificationDialog, NOTIFICATION_DIALOG_ID};
use crate::components::Action;
use crate::EventResult;

use super::state::App;

impl App {
    /// True iff the action is a MUX-001/MUX-004 mux variant.
    pub fn is_mux_action(action: &Action) -> bool {
        matches!(
            action,
            Action::MuxEnterWorkUnit(_)
                | Action::MuxConfigApplied(_)
                | Action::MuxConfigAppliedAndSaved(_)
                | Action::OpenMuxConfigDialog
        )
    }

    /// Route a mux action (store-level half).
    pub(crate) fn dispatch_mux(&mut self, action: &Action) {
        match action {
            Action::MuxEnterWorkUnit(id)
                if self.navigator.active_view == crate::views::ViewMode::Mux =>
            {
                self.handle_enter_work_unit(id);
            }
            // MUX-004: the MuxConfigDialog committed its draft. Apply the
            // draft layout string (MUX-010) and flip the enabled state
            // per R7. The 'AndSaved' variant persists FIRST (persistence
            // reads `navigator.mux.config()`).
            Action::MuxConfigApplied(draft) => self.apply_mux_config_draft(draft.clone()),
            Action::MuxConfigAppliedAndSaved(draft) => {
                self.apply_mux_config_draft(draft.clone());
                // BUG-647: the dialog is used from the board, where no
                // scrollback shows: a failed save is an app-level error.
                if let Err(err) = self.save_mux_config() {
                    let _ = self.action_tx.send(Action::SessionOperationFailed(format!(
                        "/mux: could not save config: {err}"
                    )));
                }
            }
            // MUX-009: the Board 'm'/'M' keybinding (single Board view or
            // the focused Board pane in mux mode) opens the MuxConfigDialog
            // — the exact same entry point bare /mux uses (MUX-004 R2:
            // idempotent, seeded from the live mux config).
            Action::OpenMuxConfigDialog => self.handle_open_mux_config_dialog(),
            _ => {}
        }
    }

    /// MUX-013: run a mux bar action. New Agent / New Bot open their
    /// dialogs with the MUX-002 placement armed (the new session fills the
    /// next agent slot, or the last one through a window step, and takes
    /// the focus; Cancel disarms it); Close opens TUI-124's close dialog
    /// for the focused pane's session; anything else runs as on the board.
    pub(crate) fn run_mux_bar_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::NewAgent => {
                self.navigator.mux.request_new_agent();
                self.handle_open_create_session_dialog(None);
            }
            MenuAction::NewBot => {
                self.navigator.mux.request_new_agent();
                self.handle_open_new_bot_dialog();
            }
            MenuAction::CloseAgent => {
                // The store follows the focused pane first: the dialog
                // opens only for the current session.
                self.sync_mux_focus_to_session();
                if let Some(session) = self.navigator.mux.focused_session_id() {
                    self.request_close(session);
                }
            }
            other => self.dispatch(other.to_action()),
        }
    }

    /// Push a one-line mux error into the focused session's scrollback
    /// (R7). BUG-647: with no session it is an app-level error, never
    /// dropped (/mux commands run only in the current session, BUG-552).
    pub(crate) fn push_mux_error(&mut self, message: &str) {
        match self.agent_view_store.current_session_context_mut() {
            Some(context) => context.push_line(message),
            None => {
                let _ = self
                    .action_tx
                    .send(Action::SessionOperationFailed(message.to_string()));
            }
        }
    }

    /// Push a one-line mux notice into the focused session's scrollback.
    /// BUG-647: with no session it is an info notification that stays
    /// until dismissed, never dropped.
    pub(crate) fn push_mux_notice(&mut self, message: &str) {
        match self.agent_view_store.current_session_context_mut() {
            Some(context) => context.push_line(message),
            None => {
                let _ = self.compositor.remove(NOTIFICATION_DIALOG_ID);
                self.compositor.push(Box::new(
                    NotificationDialog::info(message)
                        .with_auto_dismiss_ms(0)
                        .with_action_tx(self.action_tx.clone()),
                ));
            }
        }
    }

    /// MUX-002: re-sync the mux agent window to the current open-session
    /// list after any session add/remove. Clamps `window_start` and
    /// re-derives the rendered pane list (unfilled agent slots are
    /// dropped; the window re-clamps when a session is closed).
    pub(crate) fn mux_sync_window(&mut self) {
        let mux = &self.navigator.mux;
        if !mux.config().enabled {
            return;
        }
        let ids: Vec<codelet_rpc_types::SessionId> = self
            .agent_view_store
            .open_sessions()
            .iter()
            .map(|c| c.id.clone())
            .collect();
        self.navigator.mux.sync_window(&ids);
    }

    /// MUX-002/MUX-010: true for the Shift+arrows the mux claims: always
    /// Shift+Left/Right; Shift+Up/Down only when a pane lies above /
    /// below the focused one and no exec stdin box is focused — otherwise
    /// they stay the pane's keys (agent prompt / stdin history recall).
    pub(crate) fn is_mux_shift_key(&self, key: &crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        if !key.modifiers.contains(KeyModifiers::SHIFT) {
            return false;
        }
        match key.code {
            KeyCode::Left | KeyCode::Right => true,
            KeyCode::Up | KeyCode::Down => {
                !self.navigator.agent.stdin_box_focused()
                    && self
                        .navigator
                        .mux
                        .has_vertical_neighbour(key.code == KeyCode::Down)
            }
            _ => false,
        }
    }

    /// MUX-002/MUX-010/MUX-012: Shift+arrows in mux mode — Left/Right move
    /// focus in layout order and prompt for a new agent on a last agent
    /// pane, Up/Down move it spatially. Runs at the
    /// App level (before the Navigator) so the CreateSessionDialog opens
    /// synchronously on the same event; the Navigator's mux Shift+
    /// arrow arms are unreachable while mux is enabled.
    pub(crate) fn handle_mux_shift_key(&mut self, key: crossterm::event::KeyEvent) -> EventResult {
        match key.code {
            crossterm::event::KeyCode::Right => {
                if self.navigator.mux.shift_right() {
                    self.handle_open_create_session_dialog(None);
                }
                self.sync_mux_focus_to_session();
                EventResult::consumed()
            }
            crossterm::event::KeyCode::Left => {
                self.navigator.mux.shift_left();
                self.sync_mux_focus_to_session();
                EventResult::consumed()
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down => {
                let down = key.code == crossterm::event::KeyCode::Down;
                self.navigator.mux.shift_vertical(down);
                self.sync_mux_focus_to_session();
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    /// BUG-163 — keep the store's current session in lockstep with the
    /// mux's focused agent pane. The single live `MultiLineInput` always
    /// holds the FOCUSED session's draft (RPC-052 mirror / RPC-024
    /// round-trip), so when mux focus lands on a different agent pane
    /// (click-to-focus, Shift+Left/Right, a chip's window rotation) the store's
    /// `current_session_index` must follow: the outgoing draft is
    /// snapshotted into the old session's `input_draft`, the incoming
    /// session's persisted draft is restored into the live composer,
    /// and supervisor badges are reloaded for the incoming session.
    ///
    /// Called from `App::handle_event` AFTER Navigator routing while
    /// mux is enabled, so every focus-change path (mouse click, shift
    /// move) converges here. No-op when the focused pane is not an
    /// agent pane (the live composer keeps its session) or already
    /// matches the store's current session.
    pub(crate) fn sync_mux_focus_to_session(&mut self) {
        let mux = &self.navigator.mux;
        if !mux.config().enabled {
            return;
        }
        let Some(target) = mux.focused_session_id() else {
            return;
        };
        if self.agent_view_store.current_session() == Some(&target) {
            return;
        }
        if let Some(idx) = self
            .agent_view_store
            .open_sessions()
            .iter()
            .position(|c| c.id == target)
        {
            self.switch_to_session_index(idx);
        }
    }

    /// BUG-165: true iff mux mode is active AND the focused pane is the
    /// Board pane. Used by the App-level Esc fallback (RPC-102) so the
    /// "Exit fspec?" confirmation dialog opens from the board pane in
    /// mux mode exactly as it does from the single Board view (R9:
    /// dialogs overlay the mux). With no open agents the agent slots
    /// are dropped from the effective panes, the Board pane is the only
    /// (and focused) pane — pre-fix Esc was a dead key there because
    /// the Stage-4 guard only matched `ViewMode::Board`.
    pub(crate) fn mux_board_pane_focused(&self) -> bool {
        let mux = &self.navigator.mux;
        if !mux.config().enabled {
            return false;
        }
        mux.effective_panes()
            .get(mux.focus())
            .is_some_and(|kind| *kind == crate::views::multiplex::MuxPaneKind::Board)
    }
}
