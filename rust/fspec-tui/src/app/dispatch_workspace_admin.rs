//! TUI-122: adding and removing workspaces from the board dropdown.
//!
//! Feature: spec/features/add-and-remove-workspaces-from-the-board-dropdown.feature

use codelet_rpc_types::WorkspaceEntry;

use super::App;
use crate::components::add_workspace_dialog::{AddWorkspaceDialog, ADD_WORKSPACE_DIALOG_ID};
use crate::components::Action;

impl App {
    pub(crate) fn try_dispatch_workspace_admin(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenAddWorkspaceDialog => {
                self.board_store.close_workspace_dropdown();
                self.open_add_workspace_dialog("", None);
            }
            Action::AddWorkspace(path) => self.spawn_add_workspace(path.clone()),
            Action::WorkspaceAddFailed { path, error } => {
                self.open_add_workspace_dialog(path, Some(error.clone()));
            }
            Action::WorkspaceAdded(entry) => self.handle_workspace_added(entry.clone()),
            Action::RemoveWorkspace => self.handle_remove_workspace(),
            Action::WorkspaceRemoved(path) => self.handle_workspace_removed(path),
            _ => return false,
        }
        true
    }

    fn open_add_workspace_dialog(&mut self, path: &str, error: Option<String>) {
        let _ = self.compositor.remove(ADD_WORKSPACE_DIALOG_ID);
        self.compositor.push(Box::new(AddWorkspaceDialog::new(
            path,
            error,
            self.action_tx.clone(),
        )));
    }

    fn spawn_add_workspace(&mut self, path: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let _ = tx.send(match backend.add_workspace(path.clone()).await {
                Ok(entry) => Action::WorkspaceAdded(entry),
                Err(error) => Action::WorkspaceAddFailed {
                    path,
                    error: format!("{error:#}"),
                },
            });
        }));
    }

    /// List the added workspace and show it on the board.
    fn handle_workspace_added(&mut self, entry: WorkspaceEntry) {
        let mut entries = self.board_store.workspaces().to_vec();
        if !entries.iter().any(|known| known.path == entry.path) {
            entries.push(entry.clone());
        }
        self.board_store.set_workspaces(entries);
        self.board_store.select_workspace(&entry.path);
        self.apply_workspace_selection();
    }

    /// Remove the highlighted workspace — never the start workspace or one
    /// with an open session (the reason is shown instead).
    fn handle_remove_workspace(&mut self) {
        let Some(entry) = self.board_store.workspace_dropdown_entry(None).cloned() else {
            return;
        };
        self.board_store.close_workspace_dropdown();
        if entry.start {
            return self.dispatch(Action::SessionOperationFailed(format!(
                "{} is the daemon's start workspace and cannot be removed",
                entry.name
            )));
        }
        let has_open_session = self.agent_view_store.open_sessions().iter().any(|session| {
            self.agent_view_store.session_project(&session.id) == Some(entry.path.as_str())
        });
        if has_open_session {
            return self.dispatch(Action::SessionOperationFailed(format!(
                "{} has an open session; close its sessions before removing the workspace",
                entry.name
            )));
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let _ = tx.send(match backend.remove_workspace(entry.path.clone()).await {
                Ok(()) => Action::WorkspaceRemoved(entry.path),
                Err(error) => Action::SessionOperationFailed(format!(
                    "The workspace {} could not be removed: {error:#}",
                    entry.name
                )),
            });
        }));
    }

    /// Drop the removed workspace; a removed selection returns the board to
    /// the start workspace.
    fn handle_workspace_removed(&mut self, path: &str) {
        let entries = self
            .board_store
            .workspaces()
            .iter()
            .filter(|entry| entry.path != path)
            .cloned()
            .collect();
        self.board_store.set_workspaces(entries);
        self.apply_workspace_selection();
    }
}
