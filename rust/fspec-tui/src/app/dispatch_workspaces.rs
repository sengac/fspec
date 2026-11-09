//! RPC-445: App::dispatch routing for the board header's workspace
//! dropdown — loading the workspace list, choosing a workspace, and
//! scoping the board's work units and session chips to it.
//!
//! Feature: spec/features/board-workspace-dropdown.feature
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use codelet_rpc_types::{WorkUnitInfo, WorkspaceWorkUnits};
use tokio::sync::broadcast;
use tracing::debug;

use super::App;
use crate::components::Action;

impl App {
    pub(crate) fn try_dispatch_workspaces(&mut self, action: &Action) -> bool {
        match action {
            Action::WorkspacesLoaded(entries) => self.handle_workspaces_loaded(entries.clone()),
            // TUI-127: an empty list opens too, so `a` can add the first one.
            Action::OpenWorkspaceDropdown => {
                if self.board_pane_ready("Workspaces") {
                    self.board_store.open_workspace_dropdown();
                }
            }
            Action::CloseWorkspaceDropdown => self.board_store.close_workspace_dropdown(),
            Action::MoveWorkspaceHighlight(delta) => {
                self.board_store.move_workspace_highlight(*delta);
            }
            Action::ChooseWorkspace(row) => self.handle_choose_workspace(*row),
            Action::WorkspaceGitStateChanged(push) => self.handle_workspace_git_state_changed(push),
            Action::WorkspaceWorkUnitsLoaded(push) => {
                if self.board_store.selected_other_workspace() == Some(push.workspace.as_str()) {
                    self.apply_work_units(&push.units);
                }
            }
            // TUI-122: add / remove (app/dispatch_workspace_admin.rs).
            _ => return self.try_dispatch_workspace_admin(action),
        }
        true
    }

    /// TUI-129: the header dropdowns (Workspaces, Daemons) are painted by,
    /// and take their keys from, the board. In the mux the Board pane is
    /// focused first; a layout without a Board pane shows a notice instead
    /// of a dropdown nobody could see or close. True when it may open.
    pub(crate) fn board_pane_ready(&mut self, what: &str) -> bool {
        if self.navigator.active_view != crate::views::ViewMode::Mux {
            return true;
        }
        let board = self
            .navigator
            .mux
            .effective_panes()
            .iter()
            .position(|kind| *kind == crate::views::multiplex::MuxPaneKind::Board);
        let Some(board) = board else {
            self.dispatch(Action::SessionOperationFailed(format!(
                "{what} are shown on the Board pane: add a Board pane to the mux layout (m) to use them"
            )));
            return false;
        };
        self.navigator.mux.set_focus(board);
        true
    }

    /// The start workspace's stream: applied only while it is selected.
    pub(crate) fn handle_start_work_units_loaded(&mut self, units: &[WorkUnitInfo]) {
        if self.board_store.selected_other_workspace().is_none() {
            self.apply_work_units(units);
        }
    }

    fn apply_work_units(&mut self, units: &[WorkUnitInfo]) {
        self.board_store.replace_work_units(units.to_vec());
        // BUG-180: project the fresh snapshot onto the per-session
        // WorkUnitContext bindings (+ legacy fallback slots) so the
        // SessionHeader chip tracks the LIVE status.
        self.agent_view_store.sync_work_unit_contexts(units);
    }

    fn handle_workspaces_loaded(&mut self, entries: Vec<codelet_rpc_types::WorkspaceEntry>) {
        self.board_store.set_workspaces(entries);
        self.sync_shown_workspace();
        // Reconnect re-lists workspaces: re-read a non-start selection.
        self.spawn_load_selected_work_units();
    }

    fn handle_choose_workspace(&mut self, row: Option<usize>) {
        let Some(path) = self
            .board_store
            .workspace_dropdown_entry(row)
            .map(|entry| entry.path.clone())
        else {
            self.board_store.close_workspace_dropdown();
            return;
        };
        self.board_store.select_workspace(&path);
        self.apply_workspace_selection();
    }

    /// Show the selected workspace: its sessions, work units, checkpoint
    /// counts (RPC-453) and footer directory, branch and git state (RPC-454).
    pub(crate) fn apply_workspace_selection(&mut self) {
        self.sync_shown_workspace();
        self.spawn_load_selected_work_units();
        self.spawn_refresh_checkpoint_counts();
        self.spawn_load_selected_workspace_git();
    }

    fn sync_shown_workspace(&mut self) {
        let shown = self
            .board_store
            .selected_workspace()
            .map(|entry| entry.path.clone());
        self.agent_view_store.set_shown_workspace(shown);
    }

    /// Fetch the selected workspace's work units onto the action bus.
    fn spawn_load_selected_work_units(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        let other = self
            .board_store
            .selected_other_workspace()
            .map(str::to_owned);
        self.pending_tasks.push(tokio::spawn(async move {
            match other {
                Some(workspace) => match backend.list_workspace_work_units(workspace.clone()).await
                {
                    Ok(units) => {
                        let push = WorkspaceWorkUnits { workspace, units };
                        let _ = tx.send(Action::WorkspaceWorkUnitsLoaded(push));
                    }
                    Err(error) => {
                        let _ = tx.send(Action::SessionOperationFailed(format!(
                            "The work units of {workspace} could not be read: {error:#}"
                        )));
                    }
                },
                None => {
                    let _ = tx.send(match backend.list_work_units().await {
                        Ok(units) => Action::WorkUnitsLoaded(units),
                        Err(error) => Action::SessionOperationFailed(format!(
                            "The work units could not be read: {error:#}"
                        )),
                    });
                }
            }
        }));
    }

    /// Fetch the daemon's workspace list (bootstrap and reconnect).
    pub(crate) fn spawn_load_workspaces(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            match backend.list_workspaces().await {
                Ok(entries) => {
                    let _ = tx.send(Action::WorkspacesLoaded(entries));
                }
                // RPC-454: a failed workspace listing is shown.
                Err(error) => {
                    let _ = tx.send(Action::SessionOperationFailed(format!(
                        "The workspaces could not be listed: {error:#}"
                    )));
                }
            }
        }));
    }

    /// Fold workspace-keyed pushes onto the action bus; the dispatcher
    /// keeps only the selected workspace's.
    pub(crate) fn spawn_workspace_work_units_subscriber(&mut self) {
        let tx = self.action_tx.clone();
        let mut rx = self.backend.workspace_work_units_rx();
        self.subscriber_tasks.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(push) => {
                        let _ = tx.send(Action::WorkspaceWorkUnitsLoaded(push));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("workspace work-units subscriber lagged by {n}");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }
}
