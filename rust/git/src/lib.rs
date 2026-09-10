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
mod merge_commit;
mod repo_ops;
pub mod session_diff;
mod session_result;
mod session_status;
pub mod status;
pub mod three_way_merge;
pub mod tree_snapshot;
pub mod tree_utils;
pub mod utils;
pub mod worktree;

use std::path::Path;

pub use diff::{get_checkpoint_file_diff, get_file_diff, is_binary_file};
pub use error::{GitError, Result};
pub use repo_ops::{git_add, git_commit, git_init, git_set_config, resolve_ref};
pub use session_result::{abort_session, apply_session_changes, get_session_diff, SessionResult};
pub use session_status::{
    complete_session, create_session_manifest, delete_manifest, derive_session_status,
    discard_session, get_manifest_path, get_sessions_dir, inspect_session, is_orphaned,
    list_sessions, merge_session, merge_session_with_strategy, prune_orphaned, read_manifest,
    terminate_session, write_manifest, DerivedSessionStatus, DiscardResult, MergeResult,
    PruneResult, SessionFilter, SessionInfo, SessionManifest,
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
pub fn open_repo(dir: impl AsRef<Path>) -> Result<gix::Repository> {
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

/// WT-008: create a commit object with an explicit identity and
/// advance HEAD to it.
///
/// Used by `merge_commit::commit_merged_session` — the merge commit
/// parents on the current HEAD and updates the checked-out branch
/// ref (or the HEAD file when detached). Unlike
/// [`repo_ops::git_commit`], the author/committer identity is passed
/// explicitly (resolved from the repo-local git config by the caller)
/// rather than hardcoded.
pub(crate) fn git_commit_signature(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    parent: &gix::ObjectId,
    author_name: &str,
    author_email: &str,
    message: &str,
) -> Result<String> {
    use smallvec::SmallVec;

    let now = chrono::Utc::now();
    let time = gix::date::Time::new(now.timestamp(), 0i32);
    let signature = gix::actor::Signature {
        name: gix::bstr::BString::from(author_name),
        email: gix::bstr::BString::from(author_email),
        time,
    };

    let parents: SmallVec<[gix::ObjectId; 1]> = SmallVec::from_buf([*parent]);
    let commit = gix::objs::Commit {
        tree: tree_id,
        parents,
        author: signature.clone(),
        committer: signature,
        encoding: None,
        message: gix::bstr::BString::from(message),
        extra_headers: vec![],
    };

    let commit_id = repo
        .write_object(gix::objs::Object::Commit(commit))
        .map_err(|e| GitError::Other(format!("Failed to write commit: {e}")))?;

    // Advance HEAD: a normal checkout's HEAD is a ref (refs/heads/...),
    // a detached HEAD is a raw SHA in the HEAD file.
    let head_ref = repo
        .head_ref()
        .map_err(|e| GitError::Head(format!("Failed to read HEAD: {e}")))?;
    if let Some(mut head_ref) = head_ref {
        head_ref
            .set_target_id(commit_id, "commit")
            .map_err(|e| GitError::Other(format!("Failed to update HEAD: {e}")))?;
    } else {
        let head_path = repo
            .workdir()
            .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
            .join(".git")
            .join("HEAD");
        std::fs::write(head_path, format!("{commit_id}\n"))?;
    }

    Ok(commit_id.to_string())
}
