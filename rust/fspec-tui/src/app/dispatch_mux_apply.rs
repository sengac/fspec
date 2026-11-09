//! MUX-001/MUX-004 — `/mux` subcommand apply + MuxConfigDialog draft
//! application.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Factored out of `app/dispatch_mux.rs` so that file stays under the
//! 300-LoC ceiling. Hosts the parsed-subcommand applier (R1–R7), the
//! dialog-draft applier (BUG-166 scale re-derivation + R7 ON/OFF
//! transition) and the on/off handlers.

use crate::views::multiplex::layout_tree::check_fit;
use crate::views::multiplex::{MuxConfig, MuxLayout};

use super::mux_parser::{parse_mux_command, MuxSubcommand};
use super::state::App;

/// The `/mux help` notice (MUX-010: the layout grammar, weights, the kept
/// subcommands and Shift+arrow focus; MUX-012: Left/Right in layout order;
/// MUX-013: the bar buttons).
pub(crate) const MUX_HELP: &str = "/mux <layout> sets the panes, e.g. /mux agent, [board, board], \
     board, files: the top level is columns, each [..] splits the other way (rows, then columns, \
     ...), pane:N or [..]:N weights a share (default 1); panes are board, agent, files, \
     checkpoints · /mux opens the config dialog · /mux on|off|save|default|help · \
     Shift+Left/Right move to the previous/next pane in layout order, Shift+Up/Down to the pane \
     above/below · on the bar, [ New Bot ] starts a bot, [ New Agent ] starts an agent (in the \
     next agent pane) and [ Close ] closes the focused pane's bot or agent";

impl App {
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
            MuxSubcommand::Layout(layout) => {
                // MUX-010: the layout must fit the minimum pane size at
                // the current mux body; a refused layout leaves the
                // config untouched.
                if !self.mux_layout_fits(&layout) {
                    return;
                }
                self.navigator.mux.set_layout(layout);
            }
            MuxSubcommand::Save => match self.save_mux_config() {
                Ok(()) => self.push_mux_notice("[mux] saved to fspec-config.json (tui.mux)"),
                Err(err) => {
                    self.push_mux_error(&format!("/mux save: {err}"));
                }
            },
            MuxSubcommand::Default => {
                // BUG-175: enter the grid in lockstep with the live
                // view — the default preset (layout + home focus) with
                // the enabled flag ON. Entering
                // ViewMode::Mux with the flag still off would leave
                // every flag-gated path (Shift+Left/Right intercept,
                // key classification, the R6 auto-save) mis-firing
                // inside the grid.
                let config = crate::views::multiplex::MuxConfig {
                    enabled: true,
                    ..MuxConfig::default()
                };
                // MUX-010: with mux already on, keep the view mux was
                // entered from (the live view is Mux itself).
                let pre_mux = if self.navigator.mux.config().enabled {
                    self.navigator.mux.pre_mux_view().unwrap_or_default()
                } else {
                    self.navigator.active_view
                };
                self.navigator.mux.enable_with_config(config, pre_mux);
                self.navigator.active_view = crate::views::ViewMode::Mux;
            }
            MuxSubcommand::Help => {
                self.push_mux_notice(MUX_HELP);
            }
        }
        if self.navigator.mux.config().enabled {
            // BUG-201: entering (re-entering) the grid clears the
            // board store's OWN bar state — the board pane's bar is
            // suppressed in the grid (MENU-004 R-SUPPRESS), so a stale
            // `menu_focus` / `open_menu` from the single Board view
            // would make Left/Right walk that INVISIBLE ring.
            // `focused_column` stays (the entry must not reset it).
            self.board_store.dismiss_menu();
            self.navigator.active_view = crate::views::ViewMode::Mux;
            self.mux_sync_window();
            self.navigator.mux.recompute_rects();
            // BUG-182 R7: entering mux mode triggers an initial load for
            // every rendered lazy pane that has not loaded yet (panes
            // load IN PLACE — no whole-view flip).
            self.mux_load_lazy_panes();
        }
    }

    /// MUX-004 (R5/R7): apply a committed MuxConfigDialog draft to the
    /// live mux layout. The MUX-010 layout string (already parsed and
    /// fit-checked by the dialog) is applied verbatim. The draft's
    /// enabled state decides the R7 transition:
    ///   - OFF → ON: enter mux mode with the draft layout, remembering
    ///     the current active view as the pre-mux view;
    ///   - ON → OFF: apply the draft layout to the stored config, then
    ///     exit mux mode back to the pre-mux view;
    ///   - unchanged (ON→ON / OFF→OFF): only the layout refreshes.
    pub(crate) fn apply_mux_config_draft(&mut self, draft: MuxConfig) {
        let was_enabled = self.navigator.mux.config().enabled;
        let config = draft;
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
            // BUG-201: entering (re-entering) the grid clears the
            // board store's OWN bar state (ring focus + open
            // dropdown) — the board pane's bar is suppressed in the
            // grid (MENU-004 R-SUPPRESS), so a stale `menu_focus` /
            // `open_menu` from the single Board view would make
            // Left/Right walk that INVISIBLE ring. `focused_column`
            // stays (the entry must not reset it).
            self.board_store.dismiss_menu();
            self.mux_sync_window();
            self.navigator.mux.recompute_rects();
            // BUG-182 R7: OFF → ON entry loads un-loaded lazy panes.
            self.mux_load_lazy_panes();
        } else if was_enabled {
            // Exit mux mode (R7): the draft layout is recorded on the
            // stored config first so the next dialog open (and the R6
            // auto-save on exit) reflects what the user committed.
            let live = self.navigator.mux.config_mut();
            live.layout = config.layout;
            live.focused_pane = config.focused_pane;
            let view = self.navigator.mux.disable();
            self.navigator.active_view = view;
        } else {
            // Mux stayed OFF: only refresh the stored layout (the live
            // grid was untouched while the dialog was open — R5).
            self.navigator.mux.config_mut().clone_from(&config);
            // BUG-183: a committed layout replaces the saved pane list —
            // the live-only closed-pane set applies to the OLD layout
            // and must not filter the new one.
            self.navigator.mux.clear_closed_panes();
        }
    }

    /// `/mux on` — enable with the saved/default config (R1/R6).
    /// Fresh entry focuses the BOARD pane (index 0) — the view the
    /// user came from — so App-level shortcuts (`?`, `m`) still
    /// fall through from the board pane (R9).
    /// MUX-010: true when every pane of `layout` meets the minimum pane
    /// size at the current mux body; otherwise the exact error is pushed.
    fn mux_layout_fits(&mut self, layout: &MuxLayout) -> bool {
        match check_fit(self.navigator.mux.body_area(), layout) {
            Ok(()) => true,
            Err(err) => {
                self.push_mux_error(&format!("/mux: {err}"));
                false
            }
        }
    }

    fn handle_mux_on(&mut self) {
        let saved = self.navigator.mux.config().layout.clone();
        if !self.mux_layout_fits(&saved) {
            return;
        }
        let pre = self.navigator.active_view;
        self.navigator.mux.set_pre_mux_view(pre);
        self.navigator.mux.config_mut().enabled = true;
        self.navigator.mux.set_focus(0);
        self.navigator.active_view = crate::views::ViewMode::Mux;
        // BUG-201: clear the board store's OWN bar state (ring focus +
        // open dropdown) — the board pane's bar is suppressed in the
        // grid (MENU-004 R-SUPPRESS), so a stale `menu_focus` /
        // `open_menu` from the single Board view would make Left/Right
        // walk that INVISIBLE ring (the ring "loses its place").
        // `focused_column` is untouched (the entry must not reset the
        // user's column).
        self.board_store.dismiss_menu();
        // BUG-183: `/mux on` re-enters the grid from the SAVED layout —
        // clear the live-only `closed_panes` set (the panes the user
        // Esc-closed come back — the transient rule) and re-derive the
        // live rendered pane list + rects so `pane_rects()` is valid
        // before the first render (same tail as the `/mux` subcommand
        // apply).
        self.navigator.mux.clear_closed_panes();
        self.mux_sync_window();
        self.navigator.mux.recompute_rects();
        // BUG-182 R7: re-entering mux loads any un-loaded lazy pane.
        self.mux_load_lazy_panes();
    }

    /// `/mux off` — disable, return to the pre-mux view (R1).
    fn handle_mux_off(&mut self) {
        let view = self.navigator.mux.disable();
        self.navigator.active_view = view;
    }

    /// BUG-182 R7: on mux entry, trigger the initial load for every
    /// rendered lazy pane that has not loaded yet — the ChangedFiles
    /// pane runs the existing open flow (reset view + spawn
    /// changed_files) and the Checkpoints pane its (reset view + spawn
    /// list_checkpoints + re-poll counts) WITHOUT flipping the whole
    /// view (R8 semantics: panes load in place). Already-loaded views
    /// are not reset (no gratuitous reload on /mux on/off cycling; a
    /// single-view open already in flight is not double-started).
    fn mux_load_lazy_panes(&mut self) {
        let panes = self.navigator.mux.effective_panes().to_vec();
        let changed_rendered = panes.contains(&crate::views::multiplex::MuxPaneKind::ChangedFiles);
        let checkpoints_rendered =
            panes.contains(&crate::views::multiplex::MuxPaneKind::Checkpoints);
        if changed_rendered {
            let single_view = self.navigator.active_view == crate::views::ViewMode::ChangedFiles;
            if !single_view && !self.navigator.changed_files.load.is_loaded() {
                // The owned view may still be mid-initial-load from a
                // prior single-view open — the open flow resets the view
                // and spawns, exactly like the `F` key.
                self.handle_open_changed_files_view();
            }
        }
        if checkpoints_rendered {
            let single_view = self.navigator.active_view == crate::views::ViewMode::Checkpoints;
            if !single_view && !self.navigator.checkpoints.load.is_loaded() {
                self.handle_open_checkpoints_view();
            }
        }
    }
}
