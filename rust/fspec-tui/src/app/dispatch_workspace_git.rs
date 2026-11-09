//! RPC-454: the selected workspace's git state, branch and directory.
//!
//! Feature: spec/features/git-state-follows-the-selected-workspace.feature

use tokio::sync::broadcast;
use tracing::debug;

use super::App;
use crate::components::Action;

impl App {
    /// Fold workspace-keyed git-state pushes onto the action bus; the
    /// dispatcher keeps only the selected workspace's.
    pub(crate) fn spawn_workspace_git_state_subscriber(&mut self) {
        let tx = self.action_tx.clone();
        let mut rx = self.backend.workspace_git_state_rx();
        self.subscriber_tasks.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(push) => {
                        let _ = tx.send(Action::WorkspaceGitStateChanged(push));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("workspace git-state subscriber lagged by {n}");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }

    /// A workspace-keyed git-state push: applied only while that workspace
    /// is the selected (non-start) workspace.
    pub(crate) fn handle_workspace_git_state_changed(
        &mut self,
        push: &codelet_rpc_types::WorkspaceGitState,
    ) {
        if self.board_store.selected_other_workspace() == Some(push.workspace.as_str()) {
            self.handle_git_state_changed(&push.state);
        }
    }

    /// The start workspace's git-state frame: applied only while the start
    /// workspace is selected.
    pub(crate) fn handle_start_git_state_changed(&mut self, state: &codelet_rpc_types::GitState) {
        if self.board_store.selected_other_workspace().is_none() {
            self.handle_git_state_changed(state);
        }
    }

    /// Load the selected workspace's directory, branch and git state for the
    /// footer and git-driven views; a failure is shown.
    pub(crate) fn spawn_load_selected_workspace_git(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let Some(workspace) = self.selected_workspace_path() else {
            return;
        };
        // The start workspace's state applies as a start frame; another
        // workspace's as a push keyed by that workspace.
        let other = self.board_store.selected_other_workspace().is_some();
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let info = backend
                .get_workspace_info_in_workspace(workspace.clone())
                .await;
            let _ = tx.send(match info {
                Ok(info) => Action::WorkspaceInfoLoaded(info),
                Err(error) => Action::SessionOperationFailed(format!(
                    "The workspace {workspace} could not be described: {error:#}"
                )),
            });
            let state = backend.git_state_in_workspace(workspace.clone()).await;
            let _ = tx.send(match state {
                Ok(state) if other => {
                    Action::WorkspaceGitStateChanged(codelet_rpc_types::WorkspaceGitState {
                        workspace: workspace.clone(),
                        state,
                    })
                }
                Ok(state) => Action::GitStateChanged(state),
                Err(error) => Action::SessionOperationFailed(format!(
                    "The git state of {workspace} could not be read: {error:#}"
                )),
            });
        }));
    }
}
