//! RPC-364 — CheckpointsView diff-stage dispatch (factored from
//! `dispatch_checkpoints.rs` to keep both files under the 300-LoC
//! ceiling).
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! The diff stage is the third leg of the checkpoints cascade: a file
//! selection change begins a new diff stage on the LoadTracker and
//! spawns `checkpoint_file_diff(work_unit_id, name, path)`; the result
//! routes back through the action bus and the view drops stale results
//! whose key ≠ current selection.

use tokio::task::JoinHandle;

use crate::components::load_state::LoadTracker;
use crate::components::Action;

use super::state::App;

impl App {
    /// TUI-106: a file selection change begins a new diff stage.
    pub(crate) fn handle_load_checkpoint_file_diff(
        &mut self,
        work_unit_id: String,
        name: String,
        path: String,
    ) {
        self.navigator.checkpoints.load.begin_stage(
            &LoadTracker::diff_stage_key(&work_unit_id, &name, &path),
            format!("Loading diff for {path}…"),
        );
        self.navigator.checkpoints.sync_loading_label();
        self.spawn_checkpoint_file_diff(work_unit_id, name, path);
    }

    pub(crate) fn spawn_checkpoint_file_diff(
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

    /// Fold a `CheckpointFileDiffLoaded` response.
    pub(crate) fn handle_checkpoint_file_diff_loaded(
        &mut self,
        work_unit_id: &str,
        name: &str,
        path: &str,
        diff: Option<String>,
    ) {
        let view = &mut self.navigator.checkpoints;
        view.set_diff(work_unit_id, name, path, diff);
        // TUI-106: matching-key stale-drop — a late diff for a
        // de-selected file must NOT clear the in-flight stage.
        view.load
            .complete_stage(&LoadTracker::diff_stage_key(work_unit_id, name, path));
        view.sync_loading_label();
    }
}
