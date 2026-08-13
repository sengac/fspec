//! Git operations using gitoxide (gix) - a pure Rust git implementation.
//!
//! This module provides git status, diff, and branch operations without
//! requiring an external git binary.
//!
//! # Example
//!
//! ```ignore
//! use codelet_git::{get_staged_files, get_current_branch};
//!
//! let staged = get_staged_files("/path/to/repo")?;
//! let branch = get_current_branch("/path/to/repo")?;
//! ```

mod change_type;
mod checkout;
mod diff;
mod error;
pub mod ghost_commit;
mod isolated_session;
mod repo_ops;
mod session_result;
mod session_status;
pub mod status;
pub mod three_way_merge;
mod tree_utils;
pub mod utils;
pub mod worktree;

use std::path::Path;

pub use diff::{get_checkpoint_file_diff, get_file_diff, is_binary_file};
pub use error::{GitError, Result};
pub use isolated_session::IsolatedSessionInfo;
pub use repo_ops::{git_add, git_commit, git_init, git_set_config, resolve_ref};
pub use session_result::{abort_session, apply_session_changes, get_session_diff, SessionResult};
pub use session_status::{
    complete_session, create_session_manifest, delete_manifest, derive_session_status,
    discard_session, get_manifest_path, get_sessions_dir, inspect_session, is_orphaned,
    list_sessions, merge_session, prune_orphaned, read_manifest, terminate_session, write_manifest,
    DerivedSessionStatus, DiscardResult, MergeResult, PruneResult, SessionFilter, SessionInfo,
    SessionManifest,
};
pub use status::{
    get_current_branch, get_staged_files, get_staged_files_with_change_type, get_unstaged_files,
    get_unstaged_files_with_change_type, get_untracked_files, ChangeType, ChangedFileStatus,
};
pub use worktree::{
    create_worktree, create_worktree_at_ref, list_worktrees, remove_worktree, WorktreeCreateResult,
    WorktreeInfo, FSPEC_WORKTREES_DIR,
};

/// Open a git repository at the given path
///
/// This is a shared helper used by status and diff operations.
/// Expects `dir` to be the repository root (containing `.git/`).
pub(crate) fn open_repo(dir: impl AsRef<Path>) -> Result<gix::Repository> {
    let path = dir.as_ref();
    gix::open(path).map_err(|e| GitError::OpenRepository {
        path: path.to_string_lossy().to_string(),
        source: Box::new(e),
    })
}

/// Discover a git repository by walking up from the given path.
///
/// Unlike `open_repo`, this traverses parent directories to find
/// the enclosing repository — just like `git` itself does when
/// you run a command from a subdirectory.
pub(crate) fn discover_repo(dir: impl AsRef<Path>) -> Result<gix::Repository> {
    let path = dir.as_ref();
    gix::discover(path).map_err(|e| GitError::DiscoverRepository {
        path: path.to_string_lossy().to_string(),
        source: Box::new(e),
    })
}
