//! RPC-365 — App::dispatch routing for checkpoint RESTORE actions.
//!
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Split out of `dispatch_checkpoints.rs` (RPC-365) so every file stays
//! under the 300-line ceiling. Spawns the restore transport calls
//! (`restore_checkpoint_file` / `restore_checkpoint_all` from RPC-362),
//! folds the outcome back as a `RestoreCheckpointResult` that drives the
//! view's status dialog, and refreshes the board counts after success.

use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

impl App {
    /// Spawn the single-file restore transport call and fold the outcome
    /// back as `RestoreCheckpointResult`.
    pub(crate) fn spawn_restore_file(
        &mut self,
        work_unit_id: String,
        name: String,
        path: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let error = match backend
                .restore_checkpoint_file(work_unit_id.clone(), name.clone(), path.clone())
                .await
            {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            };
            let _ = action_tx.send(Action::RestoreCheckpointResult {
                work_unit_id,
                name,
                path: Some(path),
                error,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Spawn the restore-all transport call and fold the outcome back as
    /// `RestoreCheckpointResult` (no path → whole-tree restore).
    pub(crate) fn spawn_restore_all(&mut self, work_unit_id: String, name: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let error = match backend
                .restore_checkpoint_all(work_unit_id.clone(), name.clone())
                .await
            {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            };
            let _ = action_tx.send(Action::RestoreCheckpointResult {
                work_unit_id,
                name,
                path: None,
                error,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Drive the view's status dialog from a restore result and dispatch
    /// its follow-ups (diff reload + counts refresh).
    pub(crate) fn handle_restore_result(
        &mut self,
        work_unit_id: &str,
        name: &str,
        path: Option<&str>,
        error: Option<&str>,
    ) {
        let follow_ups =
            self.navigator
                .checkpoints
                .on_restore_result(work_unit_id, name, path, error);
        for action in follow_ups {
            let _ = self.action_tx.send(action);
        }
    }

    /// Re-fetch checkpoint counts after a restore changed the working
    /// tree, routing `CheckpointCountsLoaded` back onto the bus.
    pub(crate) fn spawn_refresh_checkpoint_counts(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if let Ok(counts) = backend.checkpoint_counts().await {
                let _ = action_tx.send(Action::CheckpointCountsLoaded(counts));
            }
        });
        self.pending_tasks.push(handle);
    }
}
