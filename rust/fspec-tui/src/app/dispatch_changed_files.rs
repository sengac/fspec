//! RPC-356 — App::dispatch routing for the dual-pane ChangedFilesView.
//!
//! Feature: spec/features/rust-changed-files-view.feature
//!
//! Factored into its own file (like `dispatch_blocklist.rs`) to keep
//! `app/dispatch.rs` under the 300-LoC ceiling. Mirrors the RPC-015
//! `checkpoint_counts` → `CheckpointCountsLoaded` flow: on open, spawn
//! `backend.changed_files()` and route the result back through the
//! action bus; on selection-change, spawn `backend.file_diff(path)` and
//! route `FileDiffLoaded` back. All store mutation happens on the App
//! task; the Navigator's `apply_action` arm flips `active_view` BEFORE
//! these helpers run.

use tokio::task::JoinHandle;

use codelet_rpc_types::ChangedFile;

use crate::components::load_state::LoadTracker;
use crate::components::Action;

use super::state::App;

impl App {
    /// Open the changed-files view: reset the owned view + kick off the
    /// initial `changed_files()` load. The Navigator's `apply_action`
    /// arm has already flipped `active_view` to `ChangedFiles`.
    pub(crate) fn handle_open_changed_files_view(&mut self) {
        self.navigator.changed_files = crate::views::ChangedFilesView::new();
        self.spawn_changed_files();
    }

    /// RPC-452: the board's selected workspace path (the start workspace
    /// when nothing else is selected); `None` until the workspace list loads.
    pub(crate) fn selected_workspace_path(&self) -> Option<String> {
        self.board_store
            .selected_workspace()
            .map(|entry| entry.path.clone())
    }

    /// Spawn `backend.changed_files()` and route the result into the
    /// view via `Action::ChangedFilesLoaded`.
    fn spawn_changed_files(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let workspace = self.selected_workspace_path();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match list_changed_files(backend.as_ref(), workspace).await {
                Ok(files) => {
                    let _ = action_tx.send(Action::ChangedFilesLoaded(files));
                }
                // RPC-452: the failure is shown; the view's load ends empty.
                Err(e) => {
                    let _ = action_tx.send(Action::SessionOperationFailed(format!(
                        "The changed files could not be listed: {e:#}"
                    )));
                    let _ = action_tx.send(Action::ChangedFilesLoaded(Vec::new()));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a `changed_files` response into the view, then kick off the
    /// diff load for the first (now-selected) file.
    pub(crate) fn handle_changed_files_loaded(&mut self, files: Vec<ChangedFile>) {
        let view = &mut self.navigator.changed_files;
        view.set_files(files);
        // TUI-106: the scan stage has flushed (possibly empty); the
        // cascade continues onto the diff stage iff a file is selected.
        view.load.mark_list_flushed();
        let path = view.selected_path();
        if let Some(path) = &path {
            view.load.begin_stage(
                &LoadTracker::diff_stage_key_path(path),
                format!("Loading diff for {path}…"),
            );
        }
        view.sync_loading_label();
        if let Some(path) = path {
            self.spawn_file_diff(path);
        }
    }

    /// Spawn `backend.file_diff(path)` and route the result into the
    /// view via `Action::FileDiffLoaded`.
    pub(crate) fn handle_load_file_diff(&mut self, path: String) {
        // TUI-106: a selection change begins a new diff stage.
        let view = &mut self.navigator.changed_files;
        view.load.begin_stage(
            &LoadTracker::diff_stage_key_path(&path),
            format!("Loading diff for {path}…"),
        );
        view.sync_loading_label();
        self.spawn_file_diff(path);
    }

    fn spawn_file_diff(&mut self, path: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let workspace = self.selected_workspace_path();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let diff = match workspace {
                Some(workspace) => {
                    backend
                        .file_diff_in_workspace(workspace, path.clone())
                        .await
                }
                None => backend.file_diff(path.clone()).await,
            };
            // RPC-452: a failed diff is shown, not read as "no diff".
            let diff = match diff {
                Ok(diff) => diff,
                Err(e) => {
                    let _ = action_tx.send(Action::SessionOperationFailed(format!(
                        "The diff of {path} could not be read: {e:#}"
                    )));
                    None
                }
            };
            let _ = action_tx.send(Action::FileDiffLoaded { path, diff });
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a `file_diff` response into the view (dropped when the view
    /// has since selected a different file).
    pub(crate) fn handle_file_diff_loaded(&mut self, path: &str, diff: Option<String>) {
        let view = &mut self.navigator.changed_files;
        view.set_diff(path, diff);
        // TUI-106: matching-key stale-drop — a late diff for a
        // de-selected file must NOT clear the in-flight stage.
        view.load
            .complete_stage(&LoadTracker::diff_stage_key_path(path));
        view.sync_loading_label();
    }

    /// BUG-182 R6: fold a git-state REFRESH of the changed-files list —
    /// re-apply preserving the selection by PATH (the open flow's
    /// `ChangedFilesLoaded` resets the view instead), then re-load the
    /// diff for the (re-)selected file.
    pub(crate) fn handle_git_changed_files_loaded(&mut self, files: Vec<ChangedFile>) {
        let view = &mut self.navigator.changed_files;
        view.refresh_files_preserving_selection(files);
        if let Some(path) = view.selected_path() {
            self.spawn_file_diff(path);
        }
    }

    /// Route the RPC-356 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_changed_files(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenChangedFilesView => {
                self.handle_open_changed_files_view();
            }
            Action::ChangedFilesLoaded(files) => {
                self.handle_changed_files_loaded(files.clone());
            }
            // BUG-182: git-state refresh (R6: selection preserved by path).
            Action::GitChangedFilesLoaded(files) => {
                self.handle_git_changed_files_loaded(files.clone());
            }
            Action::LoadFileDiff(path) => {
                self.handle_load_file_diff(path.clone());
            }
            Action::FileDiffLoaded { path, diff } => {
                self.handle_file_diff_loaded(path, diff.clone());
            }
            // CloseChangedFilesView has no App-side state beyond the
            // Navigator flip (handled in apply_action) — accept it so the
            // catch-all chain short-circuits.
            Action::CloseChangedFilesView => {}
            _ => return false,
        }
        true
    }
}

/// RPC-452: the board's selected workspace's changed files, or the start
/// workspace's while no workspace list is loaded (the start workspace is
/// the default selection).
pub(crate) async fn list_changed_files(
    backend: &dyn crate::FspecBackend,
    workspace: Option<String>,
) -> anyhow::Result<Vec<ChangedFile>> {
    match workspace {
        Some(workspace) => backend.changed_files_in_workspace(workspace).await,
        None => backend.changed_files().await,
    }
}
