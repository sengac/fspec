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

    /// Spawn `backend.changed_files()` and route the result into the
    /// view via `Action::ChangedFilesLoaded`.
    fn spawn_changed_files(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.changed_files().await {
                Ok(files) => {
                    let _ = action_tx.send(Action::ChangedFilesLoaded(files));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "changed_files failed");
                    let _ = action_tx.send(Action::ChangedFilesLoaded(Vec::new()));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a `changed_files` response into the view, then kick off the
    /// diff load for the first (now-selected) file.
    pub(crate) fn handle_changed_files_loaded(&mut self, files: Vec<ChangedFile>) {
        self.navigator.changed_files.set_files(files);
        if let Some(path) = self.navigator.changed_files.selected_path() {
            self.spawn_file_diff(path);
        }
    }

    /// Spawn `backend.file_diff(path)` and route the result into the
    /// view via `Action::FileDiffLoaded`.
    pub(crate) fn handle_load_file_diff(&mut self, path: String) {
        self.spawn_file_diff(path);
    }

    fn spawn_file_diff(&mut self, path: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let diff = match backend.file_diff(path.clone()).await {
                Ok(diff) => diff,
                Err(e) => {
                    tracing::warn!(error = %e, "file_diff failed");
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
        self.navigator.changed_files.set_diff(path, diff);
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
