//! BUG-182 — App::dispatch routing for the git-state push frame.
//!
//! Feature: spec/features/git-state-watcher.feature
//!
//! `Action::GitStateChanged` folds the frame onto the EXISTING
//! single-writer paths (R3: no second writer per store field):
//!
//! - BoardStore counts via `Action::CheckpointCountsLoaded` (re-sent on
//!   the bus, so the same reducer arm applies it);
//!
//! - AgentViewStore chrome-bar branch via `Action::WorkspaceInfoLoaded`
//!   (only when the branch actually CHANGED — unchanged frames must not
//!   re-send and clobber the cwd string).
//!
//! It then re-fetches the visible lazy views (R4) — Changed Files and
//! Checkpoints — dropping the refresh while a view is still in its
//! initial load (the initial-load stage is never double-started).
//!
//! BUG-184: a refresh whose list is unchanged (identical signature to
//! the list the view already displays) is dropped BEFORE the re-fetch —
//! no re-fetch, no dialog flash, no scroll/selection churn (the 10 s
//! poll's steady-state tick is a no-op on the pane).

use codelet_rpc_types::{ChangedFile, CheckpointInfo, GitState};

use crate::components::Action;
use crate::sanitize_for_terminal;
use crate::views::multiplex::MuxPaneKind;
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// BUG-182 R3/R4: fold a git-state frame onto the store + lazy views.
    pub(crate) fn handle_git_state_changed(&mut self, state: &GitState) {
        // R3: single-writer paths.
        // (a) BoardStore counts — re-route through the existing
        // CheckpointCountsLoaded reducer arm (no second writer).
        let _ = self
            .action_tx
            .send(Action::CheckpointCountsLoaded(state.checkpoint_counts));

        // (b) AgentViewStore chrome bar — re-route through
        // WorkspaceInfoLoaded ONLY when the branch changed (dedup: an
        // unchanged-branch frame must not re-send and clobber the
        // bootstrap cwd string).
        let branch_changed = self
            .agent_view_store
            .workspace()
            .and_then(|w| w.git_branch.clone())
            != state.git_branch;
        if branch_changed {
            let cwd = self
                .agent_view_store
                .workspace()
                .map(|w| w.cwd.clone())
                .unwrap_or_default();
            let _ = self.action_tx.send(Action::WorkspaceInfoLoaded(
                codelet_rpc_types::WorkspaceInfo {
                    git_branch: state.git_branch.clone(),
                    cwd,
                },
            ));
        }

        // R4: re-fetch the visible lazy views (active single view OR a
        // rendered mux pane).
        if self.is_changed_files_visible() {
            self.refresh_changed_files_view(state);
        }
        if self.is_checkpoints_visible() {
            self.refresh_checkpoints_view(state);
        }
    }

    /// True iff the Changed Files view is visible: the active single view
    /// OR a rendered mux pane of kind ChangedFiles.
    fn is_changed_files_visible(&self) -> bool {
        match self.navigator.active_view {
            ViewMode::ChangedFiles => true,
            ViewMode::Mux => self
                .navigator
                .mux
                .effective_panes()
                .contains(&MuxPaneKind::ChangedFiles),
            _ => false,
        }
    }

    /// True iff the Checkpoints view is visible: the active single view
    /// OR a rendered mux pane of kind Checkpoints.
    fn is_checkpoints_visible(&self) -> bool {
        match self.navigator.active_view {
            ViewMode::Checkpoints => true,
            ViewMode::Mux => self
                .navigator
                .mux
                .effective_panes()
                .contains(&MuxPaneKind::Checkpoints),
            _ => false,
        }
    }

    /// R4: re-fetch the changed-files list when the view is already
    /// loaded. DROPPED while the initial load is still in flight (the
    /// initial-load stage is never double-started — no extra
    /// `changed_files` RPC).
    ///
    /// BUG-184: DROPPED when the frame's file list matches the list the
    /// view already displays (identical signature — the 10 s poll's
    /// steady-state tick). The drop keeps the pane byte-stable: no
    /// re-fetch, no diff re-load, no dialog flash, no scroll/selection
    /// churn. The result of a change-bearing refresh lands via
    /// `Action::GitChangedFilesLoaded` (R6: selection preserved by
    /// path; BUG-184: scroll preserved).
    fn refresh_changed_files_view(&mut self, state: &GitState) {
        let view = &mut self.navigator.changed_files;
        if !view.load.is_loaded() {
            return;
        }
        if view.list_signature() == changed_files_signature(&state.changed_files) {
            return;
        }
        self.spawn_changed_files_refresh();
    }

    /// R4: re-fetch the checkpoint list when the view is already loaded.
    /// DROPPED while the initial load is still in flight (mirrors the
    /// changed-files rule).
    ///
    /// BUG-184: DROPPED when the frame's checkpoint list matches the
    /// list the view already displays (identical signature — the
    /// steady-state tick). Same byte-stable drop rule as
    /// `refresh_changed_files_view`. The result of a change-bearing
    /// refresh lands via `Action::GitCheckpointsLoaded` (R6: selection
    /// preserved by work-unit + name; BUG-184: scroll preserved).
    fn refresh_checkpoints_view(&mut self, state: &GitState) {
        let view = &mut self.navigator.checkpoints;
        if !view.load.is_loaded() {
            return;
        }
        if view.list_signature() == checkpoints_signature(&state.checkpoints) {
            return;
        }
        self.spawn_checkpoints_refresh();
    }

    /// BUG-182: spawn `backend.changed_files()` for a git-state refresh;
    /// the result routes back via `Action::GitChangedFilesLoaded` (as
    /// opposed to `ChangedFilesLoaded` from the open flow, which resets
    /// the view).
    fn spawn_changed_files_refresh(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let files = match backend.changed_files().await {
                Ok(files) => files,
                Err(e) => {
                    tracing::warn!(error = %e, "git-state refresh: changed_files failed");
                    Vec::new()
                }
            };
            let _ = action_tx.send(Action::GitChangedFilesLoaded(files));
        });
        self.pending_tasks.push(handle);
    }

    /// BUG-182: spawn `backend.list_checkpoints()` for a git-state
    /// refresh; the result routes back via
    /// `Action::GitCheckpointsLoaded`.
    fn spawn_checkpoints_refresh(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let list = match backend.list_checkpoints().await {
                Ok(list) => list,
                Err(e) => {
                    tracing::warn!(error = %e, "git-state refresh: list_checkpoints failed");
                    Vec::new()
                }
            };
            let _ = action_tx.send(Action::GitCheckpointsLoaded(list));
        });
        self.pending_tasks.push(handle);
    }
}

/// BUG-184: signature of a frame's `changed_files` leg — the SAME shape
/// the ChangedFilesView stores (sanitized `path` + `change_type` +
/// staged flag, in list order). The comparison against
/// `ChangedFilesView::list_signature` (recorded from the view's stored
/// rows) is therefore exact: identical lists compare equal even though
/// the frame's fields are raw (unsanitized) on the wire.
fn changed_files_signature(files: &[ChangedFile]) -> String {
    files
        .iter()
        .map(|f| {
            format!(
                "{}:{}:{}",
                sanitize_for_terminal(&f.path),
                sanitize_for_terminal(&f.change_type),
                if f.staged { "s" } else { "w" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// BUG-184: signature of a frame's `checkpoints` leg — the SAME shape
/// the CheckpointsView stores (sanitized `work_unit_id` + `name` + auto
/// flag, in list order; the timestamp leg is excluded — the row
/// rendering never shows it and the fallback "now" stamp churns on
/// every capture when no index sidecar exists).
fn checkpoints_signature(checkpoints: &[CheckpointInfo]) -> String {
    checkpoints
        .iter()
        .map(|c| {
            format!(
                "{}/{}:{}",
                sanitize_for_terminal(&c.work_unit_id),
                sanitize_for_terminal(&c.name),
                if c.is_automatic { "auto" } else { "manual" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
