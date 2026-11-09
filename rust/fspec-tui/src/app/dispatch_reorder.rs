//! RPC-017 / RPC-450 — board card reordering on the selected workspace.
//!
//! Feature: spec/features/workspace-aware-work-unit-details-and-reordering.feature
//!
//! `ReorderUp` / `ReorderDown` move the selected card within its column of
//! the workspace the board is showing (RPC-445 selection). The move task is
//! parked on `pending_tasks` like every other backend call. TUI-129: a move
//! that cannot run, or that the backend refuses, shows a notice.

use super::state::App;
use crate::components::Action;

/// Which way a card moves within its column.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reorder {
    Up,
    Down,
}

impl App {
    pub(crate) fn handle_reorder(&mut self, direction: Reorder) {
        let Some(id) = self
            .board_store
            .selected_work_unit()
            .map(|unit| unit.id.clone())
        else {
            return;
        };
        let Some(workspace) = self
            .board_store
            .selected_workspace()
            .map(|entry| entry.path.clone())
        else {
            self.dispatch(Action::SessionOperationFailed(format!(
                "{id} cannot move yet: the board's workspace is still loading"
            )));
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let result = match direction {
                Reorder::Up => {
                    backend
                        .move_work_unit_up(workspace.clone(), id.clone())
                        .await
                }
                Reorder::Down => {
                    backend
                        .move_work_unit_down(workspace.clone(), id.clone())
                        .await
                }
            };
            if let Err(error) = result {
                tracing::warn!(%workspace, %id, ?direction, %error, "move work unit failed");
                let notice = format!("Could not move {id}: {error}");
                if action_tx
                    .send(Action::SessionOperationFailed(notice))
                    .is_err()
                {
                    tracing::warn!(%id, "the move failure notice found no App to show it");
                }
            }
        }));
    }
}
