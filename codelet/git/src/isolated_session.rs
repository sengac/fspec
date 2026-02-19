//! Isolated session information for worktree-based session isolation
//!
//! GIT-019: Provides a testable, composable unit for session isolation that can be
//! used independently of BackgroundSession. Follows DRY/SOLID principles by
//! separating isolation concerns from session management.

use crate::error::Result;
use crate::worktree::{create_worktree, create_worktree_at_ref, WorktreeCreateResult};
use std::path::{Path, PathBuf};

/// Information about session isolation state
///
/// This struct is designed to be:
/// - **Testable**: Can be tested with real git repos without BackgroundSession complexity
/// - **Composable**: BackgroundSession can delegate to this for isolation concerns
/// - **Immutable**: Once created, the isolation state doesn't change
#[derive(Debug, Clone)]
pub struct IsolatedSessionInfo {
    /// Project root directory
    pub project: PathBuf,
    /// Path to worktree for isolated sessions (None if non-isolated)
    pub worktree_path: Option<PathBuf>,
    /// Base commit SHA for isolated sessions (None if non-isolated)
    pub base_commit: Option<String>,
}

impl IsolatedSessionInfo {
    /// Create info for an isolated session with a worktree at HEAD
    ///
    /// Creates a worktree at `.fspec/worktrees/<session_id>/` and returns
    /// the isolation info with worktree_path and base_commit populated.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `session_id` - Unique session identifier
    ///
    /// # Returns
    /// IsolatedSessionInfo with worktree details
    pub fn new_isolated(repo_path: impl AsRef<Path>, session_id: &str) -> Result<Self> {
        let repo_path = repo_path.as_ref();
        let result = create_worktree(repo_path, session_id)?;
        Ok(Self::from_worktree_result(repo_path, result))
    }

    /// Create info for an isolated session with a worktree at a specific commit
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `session_id` - Unique session identifier
    /// * `commit_ref` - Commit reference to base the worktree on
    ///
    /// # Returns
    /// IsolatedSessionInfo with worktree details
    pub fn new_isolated_at_ref(
        repo_path: impl AsRef<Path>,
        session_id: &str,
        commit_ref: &str,
    ) -> Result<Self> {
        let repo_path = repo_path.as_ref();
        let result = create_worktree_at_ref(repo_path, session_id, Some(commit_ref))?;
        Ok(Self::from_worktree_result(repo_path, result))
    }

    /// Create info for a non-isolated session (no worktree)
    ///
    /// # Arguments
    /// * `project` - Project root directory
    ///
    /// # Returns
    /// IsolatedSessionInfo with no worktree (worktree_path and base_commit are None)
    pub fn new_non_isolated(project: impl Into<PathBuf>) -> Self {
        Self {
            project: project.into(),
            worktree_path: None,
            base_commit: None,
        }
    }

    /// Returns the effective working directory for this session
    ///
    /// - For isolated sessions: returns the worktree path
    /// - For non-isolated sessions: returns the project root
    pub fn effective_cwd(&self) -> PathBuf {
        self.worktree_path
            .clone()
            .unwrap_or_else(|| self.project.clone())
    }

    /// Check if this session is isolated (has a worktree)
    pub fn is_isolated(&self) -> bool {
        self.worktree_path.is_some()
    }

    /// Internal helper to create from worktree result
    fn from_worktree_result(repo_path: &Path, result: WorktreeCreateResult) -> Self {
        Self {
            project: repo_path.to_path_buf(),
            worktree_path: Some(result.info.path),
            base_commit: Some(result.base_commit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_isolated_effective_cwd_returns_project() {
        let info = IsolatedSessionInfo::new_non_isolated("/project");
        assert_eq!(info.effective_cwd(), PathBuf::from("/project"));
        assert!(!info.is_isolated());
    }

    #[test]
    fn test_isolated_effective_cwd_returns_worktree() {
        let info = IsolatedSessionInfo {
            project: PathBuf::from("/project"),
            worktree_path: Some(PathBuf::from("/project/.fspec/worktrees/abc123")),
            base_commit: Some("deadbeef".to_string()),
        };
        assert_eq!(
            info.effective_cwd(),
            PathBuf::from("/project/.fspec/worktrees/abc123")
        );
        assert!(info.is_isolated());
    }
}
