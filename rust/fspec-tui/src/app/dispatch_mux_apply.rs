//! MUX-001/MUX-004 — `/mux` subcommand apply + MuxConfigDialog draft
//! application.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! Factored out of `app/dispatch_mux.rs` so that file stays under the
//! 300-LoC ceiling. Hosts the parsed-subcommand applier (R1–R7), the
//! dialog-draft applier (BUG-166 scale re-derivation + R7 ON/OFF
//! transition) and the on/off handlers.

use crate::views::multiplex::{scale_scales, MuxConfig};

use super::mux_parser::{parse_mux_command, MuxSubcommand};
use super::state::App;

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
    pub(crate) fn apply_mux_config_draft(&mut self, draft: MuxConfig) {
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
}
