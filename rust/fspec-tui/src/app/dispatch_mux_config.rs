//! MUX-004 — `App::dispatch` / slash-command routing for opening the
//! MuxConfigDialog.
//!
//! Feature: spec/features/mux-config-dialog.feature
//!
//! One responsibility:
//!
//! - `handle_open_mux_config_dialog` — push a fresh `MuxConfigDialog`
//!   at `Priority::Foreground` seeded from the live mux config
//!   (`navigator.mux.config()`). Idempotent on dialog-id collision
//!   (R2: exactly one instance, addressed by the stable
//!   `MUX_CONFIG_DIALOG_ID`). NO session guard — the dialog is
//!   app-level, not session-level.
//!
//! Factored into its own file so `app/dispatch_slash_commands.rs` stays
//! under the 300-LoC ceiling (mirrors `dispatch_role_dialog.rs`).

use crate::components::mux_config_dialog::{MuxConfigDialog, MUX_CONFIG_DIALOG_ID};

use super::state::App;

impl App {
    /// MUX-004: push a fresh `MuxConfigDialog` onto the Compositor
    /// seeded with a draft of the live mux config (R2: idempotent on
    /// reopen; R7: opening with mux OFF is allowed — the dialog shows
    /// `Enabled: Off` + the last/saved layout).
    pub(crate) fn handle_open_mux_config_dialog(&mut self) {
        if self.compositor.contains(MUX_CONFIG_DIALOG_ID) {
            return;
        }
        let draft = self.navigator.mux.config().clone();
        let dialog = MuxConfigDialog::new(draft).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }
}
