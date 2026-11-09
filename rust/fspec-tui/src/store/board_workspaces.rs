//! RPC-445: the board header's workspace selector — the daemon's
//! workspace list, the selected workspace and the dropdown highlight.
//!
//! Feature: spec/features/board-workspace-dropdown.feature

use codelet_rpc_types::WorkspaceEntry;

use super::BoardStore;

/// Workspace list + selection + dropdown state held by [`BoardStore`].
#[derive(Debug, Default)]
pub struct WorkspaceSelector {
    entries: Vec<WorkspaceEntry>,
    /// Canonical path of the selected workspace; `None` = the start one.
    selected: Option<String>,
    /// `Some(highlighted row)` while the dropdown is open.
    open: Option<usize>,
}

impl BoardStore {
    /// Every workspace the daemon reported, start workspace first.
    pub fn workspaces(&self) -> &[WorkspaceEntry] {
        &self.workspaces.entries
    }

    /// Replace the workspace list; a selection that disappeared falls
    /// back to the start workspace.
    pub fn set_workspaces(&mut self, entries: Vec<WorkspaceEntry>) {
        let selector = &mut self.workspaces;
        if let Some(path) = &selector.selected {
            if !entries.iter().any(|entry| &entry.path == path) {
                selector.selected = None;
            }
        }
        selector.entries = entries;
        if let Some(row) = selector.open {
            selector.open = Some(row.min(selector.entries.len().saturating_sub(1)));
        }
    }

    /// The selected workspace, or the start workspace when none is
    /// selected (`None` until the list has loaded).
    pub fn selected_workspace(&self) -> Option<&WorkspaceEntry> {
        let entries = &self.workspaces.entries;
        match &self.workspaces.selected {
            Some(path) => entries.iter().find(|entry| &entry.path == path),
            None => entries.iter().find(|entry| entry.start),
        }
    }

    /// The selected workspace's path when it is NOT the start workspace —
    /// the board then reads that workspace instead of the default stream.
    pub fn selected_other_workspace(&self) -> Option<&str> {
        self.selected_workspace()
            .filter(|entry| !entry.start)
            .map(|entry| entry.path.as_str())
    }

    /// Select the workspace at `path`; returns `false` when unknown.
    pub fn select_workspace(&mut self, path: &str) -> bool {
        let Some(entry) = self.workspaces.entries.iter().find(|e| e.path == path) else {
            return false;
        };
        self.workspaces.selected = (!entry.start).then(|| entry.path.clone());
        self.workspaces.open = None;
        true
    }

    /// The highlighted dropdown row while the dropdown is open.
    pub fn workspace_dropdown(&self) -> Option<usize> {
        self.workspaces.open
    }

    /// Open the dropdown highlighting the selected workspace.
    pub fn open_workspace_dropdown(&mut self) {
        let selected = self.selected_workspace().map(|entry| entry.path.clone());
        let row = self
            .workspaces
            .entries
            .iter()
            .position(|entry| Some(&entry.path) == selected.as_ref())
            .unwrap_or(0);
        self.workspaces.open = Some(row);
    }

    pub fn close_workspace_dropdown(&mut self) {
        self.workspaces.open = None;
    }

    /// Move the dropdown highlight by `delta`, clamped to the list.
    pub fn move_workspace_highlight(&mut self, delta: isize) {
        let last = self.workspaces.entries.len().saturating_sub(1);
        if let Some(row) = self.workspaces.open.as_mut() {
            *row = row.saturating_add_signed(delta).min(last);
        }
    }

    /// The workspace at dropdown `row` (the highlight when `None`).
    pub fn workspace_dropdown_entry(&self, row: Option<usize>) -> Option<&WorkspaceEntry> {
        self.workspaces.entries.get(row.or(self.workspaces.open)?)
    }
}
