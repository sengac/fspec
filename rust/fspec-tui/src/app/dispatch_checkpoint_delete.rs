//! RPC-366 — App::dispatch routing for checkpoint DELETE actions.
//!
//! Feature: spec/features/checkpoint-delete-dispatch.feature
//!
//! Split out of `dispatch_checkpoints.rs` (mirroring
//! `dispatch_checkpoint_restore.rs`) so every file stays under the
//! 300-line ceiling. Spawns the delete transport calls
//! (`delete_checkpoint` / `delete_all_checkpoints` from RPC-362), folds
//! the outcome back as a `DeleteCheckpointResult` that drives the view's
//! row removal / close, and refreshes the board counts after success.

use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

impl App {
    /// Spawn the single-checkpoint delete transport call and fold the
    /// outcome back as `DeleteCheckpointResult` (`all = false`).
    pub(crate) fn spawn_delete_checkpoint(&mut self, work_unit_id: String, name: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let error = match backend
                .delete_checkpoint(work_unit_id.clone(), name.clone())
                .await
            {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            };
            let _ = action_tx.send(Action::DeleteCheckpointResult {
                work_unit_id,
                name,
                all: false,
                error,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Spawn the delete-all transport call and fold the outcome back as
    /// `DeleteCheckpointResult` (`all = true`).
    pub(crate) fn spawn_delete_all_checkpoints(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let error = match backend.delete_all_checkpoints().await {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            };
            let _ = action_tx.send(Action::DeleteCheckpointResult {
                work_unit_id: String::new(),
                name: String::new(),
                all: true,
                error,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Drive the view from a delete result (remove row / clear list,
    /// clamp selection or close) and dispatch its follow-ups (files
    /// reload / close + counts refresh).
    pub(crate) fn handle_delete_result(
        &mut self,
        work_unit_id: &str,
        name: &str,
        all: bool,
        error: Option<&str>,
    ) {
        let follow_ups =
            self.navigator
                .checkpoints
                .on_delete_result(work_unit_id, name, all, error);
        for action in follow_ups {
            let _ = self.action_tx.send(action);
        }
    }
}
