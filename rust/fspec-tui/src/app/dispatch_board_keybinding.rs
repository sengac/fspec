//! BOARD-023 — `App::dispatch` routing for the board 'u' actions popup
//! dialog + its Help / Exit-confirmation rows.
//!
//! Feature: spec/features/board-actions-popup-dialog-triggered-by-u-key.feature
//!
//! One responsibility:
//!
//! - `handle_open_board_keybinding_dialog` — push a fresh
//!   `BoardKeybindingDialog` at `Priority::Foreground`. Idempotent on
//!   dialog-id collision (R2: exactly one instance, addressed by the
//!   stable `BOARD_KEYBINDING_DIALOG_ID`). Seeded with the selected
//!   work unit's session target (R5: the `.` New Agent row snapshots
//!   it at open time — the modal blocks board selection while open).
//! - `OpenBoardHelp` / `OpenBoardExitConfirmation` (R6) route to the
//!   SAME push helpers the stage-4 App shortcuts in `app/events.rs`
//!   use for `?` (HelpDialog::for_board) and Esc
//!   (BoardExitConfirmationDialog) — one shared helper each, no
//!   duplicated push logic (DRY).

use crate::components::board_keybinding_dialog::{
    BoardKeybindingDialog, BOARD_KEYBINDING_DIALOG_ID,
};
use crate::components::Action;

use super::state::App;

impl App {
    /// BOARD-023 R1: push a fresh `BoardKeybindingDialog` onto the
    /// Compositor seeded with the selected work unit's session target
    /// (the `.` New Agent row emits `OpenAgentView` with that
    /// snapshot). Idempotent on reopen (R2).
    pub(crate) fn handle_open_board_keybinding_dialog(&mut self) {
        if self.compositor.contains(BOARD_KEYBINDING_DIALOG_ID) {
            return;
        }
        let target = self
            .board_store
            .selected_work_unit()
            .and_then(|u| self.board_store.session_for(&u.id).cloned());
        let dialog = BoardKeybindingDialog::new(target).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// Route the BOARD-023 Action variants through their helpers.
    /// Called from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_board_keybinding(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenBoardKeybindingDialog => {
                self.handle_open_board_keybinding_dialog();
            }
            // R6: shared helpers with the stage-4 App shortcuts
            // (`?` / Esc in app/events.rs) — one push path each.
            Action::OpenBoardHelp => self.open_board_help(),
            Action::OpenBoardExitConfirmation => self.open_board_exit_confirmation(),
            _ => return false,
        }
        // NOTE: `finish_dispatch_tick` (the post-dispatch housekeeping in
        // dispatch_capability.rs) sets `should_render = true` for every
        // tick, so this arm does not need its own render flag.
        true
    }
}
