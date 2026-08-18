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
use crate::components::load_state::LoadTracker;

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
        let view = &mut self.navigator.checkpoints;
        view.set_checkpoints(list);
        // TUI-106: the list stage has flushed (possibly empty); the
        // cascade continues onto the files stage iff a checkpoint is
        // now selected.
        view.load.mark_list_flushed();
        let selection = view
            .selected_checkpoint_info()
            .map(|cp| (cp.work_unit_id.clone(), cp.name.clone()));
        if let Some((work_unit_id, name)) = &selection {
            view.load.begin_stage(
                &LoadTracker::files_stage_key(work_unit_id, name),
                format!("Loading files for {name}…"),
            );
        }
        view.sync_loading_label();
        if let Some((work_unit_id, name)) = selection {
            self.spawn_checkpoint_files(work_unit_id, name);
        }
    }

    /// TUI-109: fold a per-item checkpoint-enumeration progress frame
    /// into the CheckpointsView LoadingDialog's counter row. Stale-drop:
    /// applied ONLY while the Checkpoints view is active AND the list
    /// stage is still in flight — once `CheckpointsLoaded` has flushed
    /// the list, a late frame (broadcast lag) must never re-open the
    /// dialog. The `done` flag is ignored: `CheckpointsLoaded` (which
    /// carries the authoritative, capped list) always takes precedence.
    pub(crate) fn handle_checkpoints_progress(
        &mut self,
        progress: codelet_rpc_types::CheckpointsProgress,
    ) {
        if self.navigator.active_view != crate::views::ViewMode::Checkpoints {
            return;
        }
        let view = &mut self.navigator.checkpoints;
        if view.load.is_loaded() {
            return;
        }
        view.loading.set_progress(progress.loaded as usize, progress.total as usize);
    }

    pub(crate) fn handle_load_checkpoint_files(&mut self, work_unit_id: String, name: String) {
        // TUI-106: a selection change re-requests this checkpoint's
        // files — the cascade stages key on (work_unit_id, name).
        self.navigator.checkpoints.load.begin_stage(
            &LoadTracker::files_stage_key(&work_unit_id, &name),
            format!("Loading files for {name}…"),
        );
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
        let view = &mut self.navigator.checkpoints;
        view.set_files(work_unit_id, name, files);
        // TUI-106: stale-drop invariance — complete_stage is a no-op
        // (returns false) when the folded key differs from the
        // in-flight stage, so a late result for a de-selected
        // checkpoint never clears a fresh stage. The view's own
        // set_files key-match mirrors this on the data side.
        view.load.complete_stage(&LoadTracker::files_stage_key(work_unit_id, name));
        let next_stage = view
            .selected_checkpoint_info()
            .zip(view.first_file_path())
            .filter(|(cp, _)| {
                cp.work_unit_id == work_unit_id && cp.name == name
            })
            .map(|(cp, path)| (cp.work_unit_id.clone(), cp.name.clone(), path));
        view.sync_loading_label();
        if let Some((work_unit_id, name, path)) = next_stage {
            view.load.begin_stage(
                &LoadTracker::diff_stage_key(&work_unit_id, &name, &path),
                format!("Loading diff for {path}…"),
            );
            view.sync_loading_label();
            self.spawn_checkpoint_file_diff(work_unit_id, name, path);
        }
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
            // TUI-109: per-item progress frame from the
            // checkpoints_progress_rx subscriber. Stale-drop: once the
            // list stage has flushed (CheckpointsLoaded folded) the
            // dialog is gone and late frames must never re-open it.
            Action::CheckpointsProgress(progress) => {
                self.handle_checkpoints_progress(*progress);
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
