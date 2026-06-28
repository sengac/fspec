//! Change-type derivation for staged/unstaged files (RPC-355).
//!
//! Derives A/M/D change types from gitoxide state — index, HEAD tree, and the
//! working directory — WITHOUT shelling out to `git`. Mirrors the TS reference
//! `src/git/status.ts::getChangeType` semantics:
//! - **A** — path is staged but absent from the HEAD tree (newly added).
//! - **D** — path is indexed but missing from the working directory.
//! - **M** — otherwise (a modification).
//! - **R** — best-effort; defaults to **M** when not cheaply detectable.

use crate::error::{GitError, Result};
use crate::open_repo;
use std::path::Path;

/// Single-letter change type for a working-tree / index file.
///
/// Serialised to a one-letter `String` at the RPC boundary (see
/// `codelet_rpc_types::ChangedFile`) so the UI can map the letter to a color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Added — untracked, or staged but absent from HEAD.
    Added,
    /// Modified — content differs.
    Modified,
    /// Deleted — indexed but missing from the working directory.
    Deleted,
}

impl ChangeType {
    /// The single-letter representation used on the wire / in the UI.
    pub fn as_letter(self) -> &'static str {
        match self {
            ChangeType::Added => "A",
            ChangeType::Modified => "M",
            ChangeType::Deleted => "D",
        }
    }
}

/// One changed file with its derived change type (path is repo-relative).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFileStatus {
    /// Repo-relative path.
    pub path: String,
    /// Derived change type.
    pub change_type: ChangeType,
}

/// Get staged files (index differs from HEAD) each with a derived change type.
///
/// A staged path absent from the HEAD tree is **Added**; a staged path missing
/// from the working directory is **Deleted**; otherwise **Modified**.
pub fn get_staged_files_with_change_type(dir: impl AsRef<Path>) -> Result<Vec<ChangedFileStatus>> {
    let dir = dir.as_ref();
    let repo = open_repo(dir)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    let staged = crate::status::get_staged_files(dir)?;

    let mut head_tree = match repo.head_commit() {
        Ok(commit) => Some(commit.tree().map_err(|e| GitError::Head(e.to_string()))?),
        Err(_) => None,
    };

    let mut out = Vec::with_capacity(staged.len());
    for path in staged {
        let in_head = match &mut head_tree {
            Some(tree) => matches!(tree.lookup_entry_by_path(&path), Ok(Some(_))),
            None => false,
        };
        let exists = workdir.join(&path).exists();
        let change_type = if !exists {
            ChangeType::Deleted
        } else if !in_head {
            ChangeType::Added
        } else {
            ChangeType::Modified
        };
        out.push(ChangedFileStatus { path, change_type });
    }
    Ok(out)
}

/// Get unstaged files (working-dir differs from index) each with a change type.
///
/// A tracked/indexed path missing from the working directory is **Deleted**;
/// otherwise **Modified**. (Untracked files are surfaced separately as
/// **Added** by the combined collector.)
pub fn get_unstaged_files_with_change_type(
    dir: impl AsRef<Path>,
) -> Result<Vec<ChangedFileStatus>> {
    let dir = dir.as_ref();
    let repo = open_repo(dir)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    // get_unstaged_files only reports paths that still exist in the workdir
    // (Modified). Re-scan the index for tracked paths that vanished (Deleted).
    let mut out: Vec<ChangedFileStatus> = crate::status::get_unstaged_files(dir)?
        .into_iter()
        .map(|path| ChangedFileStatus {
            path,
            change_type: ChangeType::Modified,
        })
        .collect();

    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;
    for entry in index.entries() {
        let path = String::from_utf8_lossy(entry.path(&index)).to_string();
        if !workdir.join(&path).exists() {
            out.push(ChangedFileStatus {
                path,
                change_type: ChangeType::Deleted,
            });
        }
    }
    Ok(out)
}
