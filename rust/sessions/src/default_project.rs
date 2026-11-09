//! RPC-444: the daemon's start workspace as the session manager's default
//! project, so sessions and workspace-scoped operations stop depending on
//! the process cwd.
//!
//! Feature: spec/features/sessions-use-their-workspace.feature

use crate::SessionManager;
use codelet_rpc_types::SessionId;
use std::path::{Path, PathBuf};

impl SessionManager {
    /// Record the start workspace (RPC-445: canonical, so session projects
    /// compare equal to the workspace paths clients select by). BUG-437: a
    /// path that cannot be canonicalized is refused, and so is a second,
    /// different workspace; setting the same workspace again is a no-op.
    pub fn set_default_project(&self, project: PathBuf) -> Result<(), String> {
        let project = std::fs::canonicalize(&project).map_err(|error| {
            format!(
                "cannot use {} as the start workspace: {error}",
                project.display()
            )
        })?;
        let current = self.default_project.get_or_init(|| project.clone());
        if *current == project {
            Ok(())
        } else {
            Err(format!(
                "the start workspace is already {}; refusing to change it to {}",
                current.display(),
                project.display()
            ))
        }
    }

    /// The start workspace. BUG-437: without one this is an error — never
    /// the process cwd or ".".
    pub fn default_project(&self) -> Result<PathBuf, String> {
        self.default_project
            .get()
            .cloned()
            .ok_or_else(|| "no start workspace is configured for this session manager".to_string())
    }

    pub(crate) fn create_session_in_workspace_checked(
        &self,
        workspace: &Path,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        let project = std::fs::canonicalize(workspace)
            .ok()
            .filter(|path| path.is_dir())
            .ok_or_else(|| format!("workspace path is not a directory: {}", workspace.display()))?;
        let model = self.get_default_model().ok_or_else(|| {
            format!(
                "create_session_in_workspace declined: no default model is set: {}",
                crate::handle_creation::CHOOSE_A_DEFAULT
            )
        })?;
        crate::handle_creation::create_session_in_project(
            self,
            &model,
            project.to_string_lossy().into_owned(),
            role,
        )
    }
}
