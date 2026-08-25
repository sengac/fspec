//! BOARD-022 — App::dispatch routing for the board `/` key (work-unit
//! search dialog) and the `SelectWorkUnit` action.
//!
//! Feature: spec/features/board-search-dialog-with-tab-toggled-id-title-description-modes.feature
//!
//! Factored into its own file (like `dispatch_viewer.rs`) to keep
//! `app/dispatch.rs` under the 300-LoC ceiling. The board's `/` handler
//! only emits `Action::OpenWorkUnitSearch`; this helper pushes a
//! [`WorkUnitSearchDialog`] seeded with the current BoardStore snapshot
//! onto the compositor, and applies `Action::SelectWorkUnit(id)` to the
//! BoardStore (focus the unit's column + select the row, viewport-aware).
//!
//! RPC architecture: NO new RPC methods — the dialog filters the
//! BoardStore snapshot client-side (kept fresh by the RPC-006
//! work-units broadcast).

use crate::components::work_unit_search_dialog::{
    WorkUnitSearchDialog, WORK_UNIT_SEARCH_DIALOG_ID,
};
use crate::components::Action;

use super::state::App;

impl App {
    /// Handle `Action::OpenWorkUnitSearch`: push a [`WorkUnitSearchDialog`]
    /// seeded with the current BoardStore snapshot onto the compositor at
    /// Priority::Foreground. Idempotent on dialog id — pressing `/` while
    /// the dialog is open never stacks a second layer.
    pub(crate) fn handle_open_work_unit_search(&mut self) {
        if self.compositor.contains(WORK_UNIT_SEARCH_DIALOG_ID) {
            return;
        }
        let units = self.board_store.work_units().to_vec();
        let dialog = WorkUnitSearchDialog::new(units).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// Handle `Action::SelectWorkUnit(id)`: focus the board's column
    /// containing `id` and select the row (viewport-aware so the card is
    /// visible). No-op when the id is not in the snapshot.
    pub(crate) fn handle_select_work_unit(&mut self, id: String) {
        let vh = self.navigator.board.last_viewport_height();
        self.board_store.select_work_unit(&id, vh);
    }

    /// Route the BOARD-022 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_work_unit_search(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenWorkUnitSearch => {
                self.handle_open_work_unit_search();
            }
            Action::SelectWorkUnit(id) => {
                self.handle_select_work_unit(id.clone());
            }
            _ => return false,
        }
        true
    }
}
