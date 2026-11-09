//! WT-017: a worktree merge never merges shared spec state.
//!
//! Feature: spec/features/merging-a-worktree-never-merges-shared-spec-state.feature
//!
//! An isolated session reads and writes the main checkout's shared spec
//! state (board and ID counters, epics, prefixes, tags, foundation,
//! attachments, config, ...) through its state-root pointer (WT-015), so
//! the worktree's checked-out copies of those files are stale and nothing
//! fspec does changes them. They are kept out of the session diff, the
//! conflict check, the apply and the merge commit, using the one
//! definition of shared state, [`codelet_common::spec_paths::is_shared_state_path`].
//! A worktree copy that differs from the session's base means something
//! wrote around the resolver: the merge is refused, naming the files,
//! before anything is copied.

use std::path::Path;

use codelet_common::spec_paths::is_shared_state_path;

use crate::error::{GitError, Result};
use crate::tree_snapshot::TreeSnapshot;

/// Whether the repo-relative `path` (forward slashes) is shared state.
pub fn is_shared(path: &str) -> bool {
    is_shared_state_path(Path::new(path))
}

/// The shared-state paths whose worktree entry differs from the base
/// (added, modified, deleted, or changed in kind or executable bit),
/// sorted.
pub fn changed_shared_paths(base: &TreeSnapshot, worktree: &TreeSnapshot) -> Vec<String> {
    let mut paths: Vec<String> = base
        .all_paths()
        .into_iter()
        .chain(worktree.all_paths())
        .filter(|path| is_shared(path))
        .collect();
    paths.sort();
    paths.dedup();
    paths.retain(|path| {
        base.entry_bytes(path) != worktree.entry_bytes(path)
            || base.entry_is_blob(path) != worktree.entry_is_blob(path)
            || base.is_executable(path) != worktree.is_executable(path)
    });
    paths
}

/// Refuse a merge whose worktree changed shared state.
pub fn refuse_changed_shared_state(base: &TreeSnapshot, worktree: &TreeSnapshot) -> Result<()> {
    let files = changed_shared_paths(base, worktree);
    if files.is_empty() {
        return Ok(());
    }
    Err(GitError::SharedStateModified { files })
}

/// `snapshot` without its shared-state paths.
pub fn without_shared(mut snapshot: TreeSnapshot) -> TreeSnapshot {
    snapshot.blobs.retain(|path, _| !is_shared(path));
    snapshot.symlinks.retain(|path, _| !is_shared(path));
    snapshot.gitlinks.retain(|path, _| !is_shared(path));
    snapshot.executables.retain(|path| !is_shared(path));
    snapshot
}
