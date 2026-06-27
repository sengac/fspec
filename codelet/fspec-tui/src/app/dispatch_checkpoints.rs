//! RPC-364 — App::dispatch routing for the three-pane CheckpointsView.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! Factored into its own file (like `dispatch_changed_files.rs`) to keep
//! `app/dispatch.rs` under the 300-LoC ceiling. Mirrors the RPC-356
//! lazy-load flow with three stages: on open spawn `list_checkpoints()`;
//! on checkpoint-selection spawn `checkpoint_diff_files(work_unit_id,
//! name)`; on file-selection spawn `checkpoint_file_diff(work_unit_id,
//! name, path)`. Each result routes back through the action bus and the
//! view drops stale results whose key ≠ current selection.

use tokio::task::JoinHandle;

use codelet_rpc_types::{ChangedFile, CheckpointInfo};

use crate::components::Action;

use super::state::App;

impl App {
    /// Open the checkpoints view: reset the owned view + kick off the
    /// initial `list_checkpoints()` load. The Navigator's `apply_action`
    /// arm has already flipped `active_view` to `Checkpoints`.
    pub(crate) fn handle_open_checkpoints_view(&mut self) {
        self.navigator.checkpoints = crate::views::CheckpointsView::new();
        self.spawn_list_checkpoints();
    }

    fn spawn_list_checkpoints(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let list = match backend.list_checkpoints().await {
                Ok(list) => list,
                Err(e) => {
                    tracing::warn!(error = %e, "list_checkpoints failed");
                    Vec::new()
                }
            };
            let _ = action_tx.send(Action::CheckpointsLoaded(list));
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a `CheckpointsLoaded` response, then kick off the file load
    /// for the first (now-selected) checkpoint.
    pub(crate) fn handle_checkpoints_loaded(&mut self, list: Vec<CheckpointInfo>) {
        self.navigator.checkpoints.set_checkpoints(list);
        if let Some(cp) = self.navigator.checkpoints.selected_checkpoint_info() {
            let (work_unit_id, name) = (cp.work_unit_id.clone(), cp.name.clone());
            self.spawn_checkpoint_files(work_unit_id, name);
        }
    }

    pub(crate) fn handle_load_checkpoint_files(&mut self, work_unit_id: String, name: String) {
        self.spawn_checkpoint_files(work_unit_id, name);
    }

    fn spawn_checkpoint_files(&mut self, work_unit_id: String, name: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let files = match backend
                .checkpoint_diff_files(work_unit_id.clone(), name.clone())
                .await
            {
                Ok(files) => files,
                Err(e) => {
                    tracing::warn!(error = %e, "checkpoint_diff_files failed");
                    Vec::new()
                }
            };
            let _ = action_tx.send(Action::CheckpointFilesLoaded {
                work_unit_id,
                name,
                files,
            });
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a `CheckpointFilesLoaded` response, then kick off the diff
    /// load for the first (now-selected) file.
    pub(crate) fn handle_checkpoint_files_loaded(
        &mut self,
        work_unit_id: &str,
        name: &str,
        files: Vec<ChangedFile>,
    ) {
        self.navigator
            .checkpoints
            .set_files(work_unit_id, name, files);
        if let Some(cp) = self.navigator.checkpoints.selected_checkpoint_info() {
            if cp.work_unit_id == work_unit_id && cp.name == name {
                if let Some(path) = self.navigator.checkpoints.first_file_path() {
                    self.spawn_checkpoint_file_diff(
                        work_unit_id.to_string(),
                        name.to_string(),
                        path,
                    );
                }
            }
        }
    }

    pub(crate) fn handle_load_checkpoint_file_diff(
        &mut self,
        work_unit_id: String,
        name: String,
        path: String,
    ) {
        self.spawn_checkpoint_file_diff(work_unit_id, name, path);
    }

    fn spawn_checkpoint_file_diff(&mut self, work_unit_id: String, name: String, path: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let diff = match backend
                .checkpoint_file_diff(work_unit_id.clone(), name.clone(), path.clone())
                .await
            {
                Ok(diff) => diff,
                Err(e) => {
                    tracing::warn!(error = %e, "checkpoint_file_diff failed");
                    None
                }
            };
            let _ = action_tx.send(Action::CheckpointFileDiffLoaded {
                work_unit_id,
                name,
                path,
                diff,
            });
        });
        self.pending_tasks.push(handle);
    }

    pub(crate) fn handle_checkpoint_file_diff_loaded(
        &mut self,
        work_unit_id: &str,
        name: &str,
        path: &str,
        diff: Option<String>,
    ) {
        self.navigator
            .checkpoints
            .set_diff(work_unit_id, name, path, diff);
    }

    /// Route the RPC-364 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_checkpoints(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenCheckpointsView => {
                self.handle_open_checkpoints_view();
            }
            Action::CheckpointsLoaded(list) => {
                self.handle_checkpoints_loaded(list.clone());
            }
            Action::LoadCheckpointFiles { work_unit_id, name } => {
                self.handle_load_checkpoint_files(work_unit_id.clone(), name.clone());
            }
            Action::CheckpointFilesLoaded {
                work_unit_id,
                name,
                files,
            } => {
                self.handle_checkpoint_files_loaded(work_unit_id, name, files.clone());
            }
            Action::LoadCheckpointFileDiff {
                work_unit_id,
                name,
                path,
            } => {
                self.handle_load_checkpoint_file_diff(
                    work_unit_id.clone(),
                    name.clone(),
                    path.clone(),
                );
            }
            Action::CheckpointFileDiffLoaded {
                work_unit_id,
                name,
                path,
                diff,
            } => {
                self.handle_checkpoint_file_diff_loaded(work_unit_id, name, path, diff.clone());
            }
            // RPC-365: restore actions.
            Action::RestoreCheckpointFile {
                work_unit_id,
                name,
                path,
            } => {
                self.spawn_restore_file(work_unit_id.clone(), name.clone(), path.clone());
            }
            Action::RestoreCheckpointAll { work_unit_id, name } => {
                self.spawn_restore_all(work_unit_id.clone(), name.clone());
            }
            Action::RestoreCheckpointResult {
                work_unit_id,
                name,
                path,
                error,
            } => {
                self.handle_restore_result(work_unit_id, name, path.as_deref(), error.as_deref());
            }
            Action::RefreshCheckpointCounts => {
                self.spawn_refresh_checkpoint_counts();
            }
            // RPC-366: delete actions.
            Action::DeleteCheckpoint { work_unit_id, name } => {
                self.spawn_delete_checkpoint(work_unit_id.clone(), name.clone());
            }
            Action::DeleteAllCheckpoints => {
                self.spawn_delete_all_checkpoints();
            }
            Action::DeleteCheckpointResult {
                work_unit_id,
                name,
                all,
                error,
            } => {
                self.handle_delete_result(work_unit_id, name, *all, error.as_deref());
            }
            // CloseCheckpointsView has no App-side state beyond the
            // Navigator flip (handled in apply_action).
            Action::CloseCheckpointsView => {}
            _ => return false,
        }
        true
    }
}
