//! Git worktree operations using gitoxide
//!
//! Provides primitives for creating, removing, and listing git worktrees
//! for session isolation. Uses pure gitoxide (gix) - NO git CLI commands.

use crate::checkout::checkout_to_worktree;
use crate::error::{GitError, Result};
use crate::open_repo;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

/// Directory name for fspec worktrees within a repository
pub const FSPEC_WORKTREES_DIR: &str = ".fspec/worktrees";

/// Information about a worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Session ID this worktree belongs to
    pub session_id: String,
    /// Absolute path to the worktree directory
    pub path: PathBuf,
    /// Commit SHA the worktree HEAD points to
    pub head_commit: String,
    /// Whether the worktree is in detached HEAD mode
    pub is_detached: bool,
}

/// Result of creating a worktree
#[derive(Debug, Clone)]
pub struct WorktreeCreateResult {
    /// Information about the created worktree
    pub info: WorktreeInfo,
    /// The base commit the worktree was created from
    pub base_commit: String,
    /// When the worktree was created
    pub created_at: DateTime<Utc>,
}

/// Create a worktree for a session at HEAD
///
/// Creates a new worktree in `.fspec/worktrees/<session_id>/` based on HEAD.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
/// * `session_id` - Unique session identifier
///
/// # Returns
/// WorktreeCreateResult with worktree info and metadata
pub fn create_worktree(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<WorktreeCreateResult> {
    create_worktree_at_ref(repo_path, session_id, None)
}

/// Create a worktree for a session at a specific commit ref
///
/// Creates a new worktree in `.fspec/worktrees/<session_id>/` based on the
/// specified commit reference (or HEAD if None).
///
/// # Arguments
/// * `repo_path` - Path to the git repository
/// * `session_id` - Unique session identifier
/// * `commit_ref` - Optional commit reference (defaults to HEAD)
///
/// # Returns
/// WorktreeCreateResult with worktree info and metadata
pub fn create_worktree_at_ref(
    repo_path: impl AsRef<Path>,
    session_id: &str,
    commit_ref: Option<&str>,
) -> Result<WorktreeCreateResult> {
    let repo_path = repo_path.as_ref();
    let repo = open_repo(repo_path)?;

    let commit_id = resolve_commit_ref(&repo, commit_ref)?;
    let commit_sha = commit_id.to_string();

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    let git_dir = repo.git_dir();
    let worktree_git_dir = git_dir.join("worktrees").join(session_id);

    if worktree_path.exists() || worktree_git_dir.exists() {
        return Err(GitError::WorktreeExists {
            session_id: session_id.to_string(),
        });
    }

    fs::create_dir_all(&worktree_path)?;
    fs::create_dir_all(&worktree_git_dir)?;

    write_worktree_metadata(&worktree_path, &worktree_git_dir, &commit_sha)?;
    checkout_to_worktree(&repo, &worktree_path, &commit_id)?;

    Ok(WorktreeCreateResult {
        info: WorktreeInfo {
            session_id: session_id.to_string(),
            path: worktree_path,
            head_commit: commit_sha.clone(),
            is_detached: true,
        },
        base_commit: commit_sha,
        created_at: Utc::now(),
    })
}

/// Remove a worktree for a session
///
/// Removes the worktree directory and cleans up git metadata.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
/// * `session_id` - Session identifier of the worktree to remove
pub fn remove_worktree(repo_path: impl AsRef<Path>, session_id: &str) -> Result<()> {
    let repo_path = repo_path.as_ref();
    let repo = open_repo(repo_path)?;

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    let git_dir = repo.git_dir();
    let worktree_git_dir = git_dir.join("worktrees").join(session_id);

    if !worktree_path.exists() && !worktree_git_dir.exists() {
        return Err(GitError::WorktreeNotFound {
            session_id: session_id.to_string(),
        });
    }

    if worktree_path.exists() {
        fs::remove_dir_all(&worktree_path)?;
    }
    if worktree_git_dir.exists() {
        fs::remove_dir_all(&worktree_git_dir)?;
    }

    Ok(())
}

/// List all session worktrees in a repository
///
/// Returns information about all worktrees in `.fspec/worktrees/`.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// Vector of WorktreeInfo for each worktree found
pub fn list_worktrees(repo_path: impl AsRef<Path>) -> Result<Vec<WorktreeInfo>> {
    let repo_path = repo_path.as_ref();
    let _repo = open_repo(repo_path)?;

    let worktrees_dir = repo_path.join(FSPEC_WORKTREES_DIR);
    if !worktrees_dir.exists() {
        return Ok(Vec::new());
    }

    let mut worktrees = Vec::new();
    for entry in fs::read_dir(&worktrees_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let session_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if session_id.is_empty() {
            continue;
        }

        let head_path = repo_path
            .join(".git/worktrees")
            .join(&session_id)
            .join("HEAD");

        let (head_commit, is_detached) = read_worktree_head(&head_path);

        worktrees.push(WorktreeInfo {
            session_id,
            path,
            head_commit,
            is_detached,
        });
    }

    Ok(worktrees)
}

// =============================================================================
// Helper functions
// =============================================================================

/// Resolve a commit reference to an object ID
fn resolve_commit_ref(repo: &gix::Repository, commit_ref: Option<&str>) -> Result<gix::ObjectId> {
    let reference = commit_ref.unwrap_or("HEAD");
    let id =
        repo.rev_parse_single(reference.as_bytes())
            .map_err(|_| GitError::InvalidCommitRef {
                commit_ref: reference.to_string(),
            })?;
    Ok(id.detach())
}

/// Write worktree metadata files
fn write_worktree_metadata(
    worktree_path: &Path,
    worktree_git_dir: &Path,
    commit_sha: &str,
) -> Result<()> {
    fs::write(worktree_git_dir.join("HEAD"), format!("{}\n", commit_sha))?;

    let worktree_gitfile = worktree_path.join(".git");
    fs::write(
        worktree_git_dir.join("gitdir"),
        format!("{}\n", worktree_gitfile.display()),
    )?;
    fs::write(worktree_git_dir.join("commondir"), "../..\n")?;
    fs::write(
        &worktree_gitfile,
        format!("gitdir: {}\n", worktree_git_dir.display()),
    )?;

    Ok(())
}

/// Read HEAD commit and detached state from worktree
fn read_worktree_head(head_path: &Path) -> (String, bool) {
    if !head_path.exists() {
        return ("unknown".to_string(), true);
    }

    match fs::read_to_string(head_path) {
        Ok(content) => {
            let content = content.trim();
            if content.starts_with("ref:") {
                (content.to_string(), false)
            } else {
                (content.to_string(), true)
            }
        }
        Err(_) => ("unknown".to_string(), true),
    }
}
