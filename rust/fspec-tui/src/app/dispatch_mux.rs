//! MUX-001 — `App::dispatch` arms for the mux `Action` variants.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Store-level work (config application, persistence, error notices)
//! lives here; the Navigator's `apply_action` handles the view flip +
//! focus. `App::dispatch` runs this arm and then calls
//! `navigator.apply_action`, so the two halves compose per action.

use crate::components::Action;
use crate::views::multiplex::{scale_scales, MuxConfig};
use crate::EventResult;

use super::mux_parser::{parse_mux_command, MuxSubcommand};
use super::state::App;

impl App {
    /// True iff the action is a MUX-001/MUX-004 mux variant.
    pub fn is_mux_action(action: &Action) -> bool {
        matches!(
            action,
            Action::MuxEnterWorkUnit(_)
                | Action::MuxConfigApplied(_)
                | Action::MuxConfigAppliedAndSaved(_)
        )
    }

    /// Route a mux action (store-level half).
    pub(crate) fn dispatch_mux(&mut self, action: &Action) {
        match action {
            Action::MuxEnterWorkUnit(id) if self.navigator.mux.config().enabled => {
                // R8: bind the work unit to the agent pane session
                // WITHOUT flipping the whole view (the Navigator's
                // apply_action focuses the agent pane instead).
                let status = self
                    .board_store
                    .column_units(self.board_store.focused_column())
                    .iter()
                    .find(|u| u.id == *id)
                    .map(|u| u.status.clone());
                self.agent_view_store
                    .set_current_work_unit(Some(id.clone()), status);
                let _ = self
                    .action_tx
                    .send(Action::AttachWorkUnitToSession(id.clone()));
            }
            // MUX-004: the MuxConfigDialog committed its draft. Apply the
            // draft layout (orientation/panes; the scale re-derives for
            // the new pane count per BUG-166) and flip the enabled state
            // per R7. The 'AndSaved' variant persists FIRST (persistence
            // reads `navigator.mux.config()`).
            Action::MuxConfigApplied(draft) => self.apply_mux_config_draft(draft.clone()),
            Action::MuxConfigAppliedAndSaved(draft) => {
                self.apply_mux_config_draft(draft.clone());
                match self.save_mux_config() {
                    Ok(()) => {}
                    Err(err) => {
                        self.push_mux_error(&format!("/mux: could not save config: {err}"));
                    }
                }
            }
            _ => {}
        }
    }

    /// Apply a parsed `/mux` subcommand (R1–R7). Parse errors surface
    /// as a one-line scrollback notice and leave the config untouched.
    pub(crate) fn handle_mux_subcommand(&mut self, line: &str) {
        match parse_mux_command(line) {
            Ok(sub) => self.apply_mux_subcommand(sub),
            Err(err) => {
                // R7: one-line error, config untouched.
                self.push_mux_error(&err.to_string());
            }
        }
    }

    fn apply_mux_subcommand(&mut self, sub: MuxSubcommand) {
        match sub {
            MuxSubcommand::On | MuxSubcommand::Off => {
                // Apply directly in this dispatch tick (synchronous
                // tests assert the view flip immediately).
                match sub {
                    MuxSubcommand::On => self.handle_mux_on(),
                    MuxSubcommand::Off => self.handle_mux_off(),
                    _ => {}
                }
            }
            MuxSubcommand::Config => {
                // MUX-004: bare /mux opens the MuxConfigDialog (R1) —
                // the on/off toggle now lives inside the dialog's
                // Enabled row. Idempotent while a dialog is open.
                self.handle_open_mux_config_dialog();
            }
            MuxSubcommand::Orientation(orientation) => {
                self.navigator.mux.set_orientation(orientation);
            }
            MuxSubcommand::PaneCount(count) => {
                self.navigator.mux.set_pane_count(count);
            }
            MuxSubcommand::PaneList {
                panes,
                split_percent,
            } => {
                self.navigator.mux.set_pane_list(panes, split_percent);
            }
            MuxSubcommand::Save => match self.save_mux_config() {
                Ok(()) => self.push_mux_notice("[mux] saved to fspec-config.json (tui.mux)"),
                Err(err) => {
                    self.push_mux_error(&format!("/mux save: {err}"));
                }
            },
            MuxSubcommand::Default => {
                // BUG-175: enter the grid in lockstep with the live
                // view — the default preset (orientation/splits/panes/
                // home focus) with the enabled flag ON. Entering
                // ViewMode::Mux with the flag still off would leave
                // every flag-gated path (Shift+Left/Right intercept,
                // key classification, the R6 auto-save) mis-firing
                // inside the grid.
                let config = crate::views::multiplex::MuxConfig {
                    enabled: true,
                    ..MuxConfig::default()
                };
                self.navigator
                    .mux
                    .enable_with_config(config, self.navigator.active_view);
                self.navigator.active_view = crate::views::ViewMode::Mux;
            }
            MuxSubcommand::Help => {
                // MUX-004: bare /mux opens the config dialog; on/off are
                // the explicit toggle subcommands.
                self.push_mux_notice(
                    "/mux opens the config dialog (panes/orientation/enabled) · \
                     /mux on|off toggle · /mux [h|v|2..4|<kinds> [pct]|save|default|help]",
                );
            }
        }
        // MUX-002: `/mux` (re)applies the grid — enter/keep mux view,
        // re-sync the agent window to the live open-session list
        // (unfilled agent slots are dropped; the window re-clamps) and
        // recompute the pane rects so `pane_rects()` is valid BEFORE
        // the first render.
        if self.navigator.mux.config().enabled {
            self.navigator.active_view = crate::views::ViewMode::Mux;
            self.mux_sync_window();
            self.navigator.mux.recompute_rects();
        }
    }

    /// MUX-004 (R5/R7): apply a committed MuxConfigDialog draft to the
    /// live mux layout. The orientation + pane list are applied
    /// verbatim; the BUG-166 percentage scale re-derives for the new
    /// pane count (`scale_scales` keeps equal scales equal; layout-only
    /// scope — split percents are never hand-edited in the dialog). The
    /// draft's enabled state decides the R7 transition:
    ///   - OFF → ON: enter mux mode with the draft layout, remembering
    ///     the current active view as the pre-mux view;
    ///   - ON → OFF: apply the draft layout to the stored config, then
    ///     exit mux mode back to the pre-mux view;
    ///   - unchanged (ON→ON / OFF→OFF): only the layout refreshes.
    fn apply_mux_config_draft(&mut self, draft: MuxConfig) {
        let was_enabled = self.navigator.mux.config().enabled;
        let mut config = draft;
        config.splits = scale_scales(&config.splits, config.panes.len());
        if config.enabled {
            let pre_mux = if was_enabled {
                self.navigator.mux.pre_mux_view().unwrap_or_default()
            } else {
                self.navigator.active_view
            };
            // Enter mux mode (or layout-refresh while already in) with
            // the draft layout (R7).
            self.navigator.mux.enable_with_config(config, pre_mux);
            self.navigator.active_view = crate::views::ViewMode::Mux;
            self.mux_sync_window();
            self.navigator.mux.recompute_rects();
        } else if was_enabled {
            // Exit mux mode (R7): the draft layout is recorded on the
            // stored config first so the next dialog open (and the R6
            // auto-save on exit) reflects what the user committed.
            let live = self.navigator.mux.config_mut();
            live.orientation = config.orientation;
            live.panes = config.panes;
            live.splits = config.splits;
            live.focused_pane = config.focused_pane;
            let view = self.navigator.mux.disable();
            self.navigator.active_view = view;
        } else {
            // Mux stayed OFF: only refresh the stored layout (the live
            // grid was untouched while the dialog was open — R5).
            self.navigator.mux.config_mut().clone_from(&config);
        }
    }

    /// `/mux on` — enable with the saved/default config (R1/R6).
    /// Fresh entry focuses the BOARD pane (index 0) — the view the
    /// user came from — so App-level shortcuts (`?`, `m`) still
    /// fall through from the board pane (R9).
    fn handle_mux_on(&mut self) {
        let pre = self.navigator.active_view;
        self.navigator.mux.set_pre_mux_view(pre);
        self.navigator.mux.config_mut().enabled = true;
        self.navigator.mux.set_focus(0);
        self.navigator.active_view = crate::views::ViewMode::Mux;
    }

    /// `/mux off` — disable, return to the pre-mux view (R1).
    fn handle_mux_off(&mut self) {
        let view = self.navigator.mux.disable();
        self.navigator.active_view = view;
    }

    /// Push a one-line mux error into the focused session's scrollback
    /// (R7). No session → silent no-op.
    fn push_mux_error(&mut self, message: &str) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        self.navigator
            .agent
            .push_line(&mut self.agent_view_store, message.to_string());
        let _ = session;
    }

    /// Push a one-line mux notice into the focused session's scrollback.
    fn push_mux_notice(&mut self, message: &str) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        self.navigator
            .agent
            .push_line(&mut self.agent_view_store, message.to_string());
        let _ = session;
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

    /// MUX-002: Shift+Left / Shift+Right in mux mode — drive the agent
    /// window and prompt for a new agent at the right edge. Runs at the
    /// App level (before the Navigator) so the CreateSessionDialog opens
    /// synchronously on the same event; the Navigator's mux Shift+
    /// arrow arms are unreachable while mux is enabled.
    pub(crate) fn handle_mux_shift_key(&mut self, key: crossterm::event::KeyEvent) -> EventResult {
        match key.code {
            crossterm::event::KeyCode::Right => {
                if self.navigator.mux.shift_right() {
                    self.handle_open_create_session_dialog(None);
                }
                EventResult::consumed()
            }
            crossterm::event::KeyCode::Left => {
                self.navigator.mux.shift_left();
                EventResult::consumed()
            }
            _ => EventResult::ignored(),
        }
    }

    /// BUG-163 — keep the store's current session in lockstep with the
    /// mux's focused agent pane. The single live `MultiLineInput` always
    /// holds the FOCUSED session's draft (RPC-052 mirror / RPC-024
    /// round-trip), so when mux focus lands on a different agent pane
    /// (click-to-focus, Shift+Left/Right window rotation) the store's
    /// `current_session_index` must follow: the outgoing draft is
    /// snapshotted into the old session's `input_draft`, the incoming
    /// session's persisted draft is restored into the live composer,
    /// and supervisor badges are reloaded for the incoming session.
    ///
    /// Called from `App::handle_event` AFTER Navigator routing while
    /// mux is enabled, so every focus-change path (mouse click, shift
    /// rotation) converges here. No-op when the focused pane is not an
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
