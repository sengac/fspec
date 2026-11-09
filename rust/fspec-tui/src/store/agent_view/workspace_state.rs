//! RPC-445: which workspace each open session belongs to, and the
//! workspace the board is showing — sessions from other workspaces are
//! hidden from the menu-bar chips.
//!
//! Feature: spec/features/board-workspace-dropdown.feature

use std::collections::HashMap;

use codelet_rpc_types::SessionId;

use super::AgentViewStore;

#[derive(Debug, Default)]
pub struct SessionWorkspaces {
    project_by_session: HashMap<SessionId, String>,
    /// Canonical path of the workspace the board shows; `None` = no filter.
    shown: Option<String>,
}

impl AgentViewStore {
    /// Record the project `session` was created in (`SessionInfo.project`).
    pub fn set_session_project(&mut self, session: SessionId, project: String) {
        if project.is_empty() {
            return;
        }
        self.session_workspaces
            .project_by_session
            .insert(session, project);
    }

    /// RPC-455: the project `session` was created in, when known.
    pub fn session_project(&self, session: &SessionId) -> Option<&str> {
        self.session_workspaces
            .project_by_session
            .get(session)
            .map(String::as_str)
    }

    /// Show only sessions of the workspace at `path` (`None` shows all).
    pub fn set_shown_workspace(&mut self, path: Option<String>) {
        self.session_workspaces.shown = path;
    }

    /// Whether `session` belongs to the shown workspace. Sessions whose
    /// project is unknown stay visible.
    pub fn session_in_shown_workspace(&self, session: &SessionId) -> bool {
        let workspaces = &self.session_workspaces;
        match (
            &workspaces.shown,
            workspaces.project_by_session.get(session),
        ) {
            (Some(shown), Some(project)) => shown == project,
            _ => true,
        }
    }
}
