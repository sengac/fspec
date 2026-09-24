//! Change-type derivation for staged/unstaged files (RPC-355).
//!
//! Derives A/M/D change types from gitoxide state — index, HEAD tree, and the
//! working directory — WITHOUT shelling out to `git`. Mirrors the TS reference
//! `src/git/status.ts::getChangeType` semantics:
//! - **A** — path is staged but absent from the HEAD tree (newly added).
//! - **D** — path is indexed but missing from the working directory.
//! - **M** — otherwise (a modification).
//! - **R** — best-effort; defaults to **M** when not cheaply detectable.
//!
//! BUG-189 R2: the in-HEAD verdict is computed ONCE — inside the single-pass
//! staged detection in `crate::status` — and reused here. This module never
//! issues its own tree lookups: the double-pass pack re-decode is gone.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::status::{staged_files_with_repo, unstaged_files_with_repo, StagedFileVerdict};
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

/// Derive the staged A/M/D letters from the single-pass verdicts (BUG-189
/// R2: the in-HEAD verdict is REUSED from the staged-detection pass — no
/// second tree lookup is ever issued for a staged path).
fn staged_change_types(verdicts: Vec<StagedFileVerdict>, workdir: &Path) -> Vec<ChangedFileStatus> {
    let mut out = Vec::with_capacity(verdicts.len());
    for verdict in verdicts {
        let exists = workdir.join(&verdict.path).exists();
        // A staged deletion (null index OID on a path that exists in HEAD)
        // is Deleted regardless of the workdir state — git reports `D` for
        // `git add --remove <file>` even while the file is still on disk.
        let change_type = if verdict.deleted_from_index || !exists {
            ChangeType::Deleted
        } else if !verdict.in_head {
            ChangeType::Added
        } else {
            ChangeType::Modified
        };
        out.push(ChangedFileStatus {
            path: verdict.path,
            change_type,
        });
    }
    out
}

/// Get staged files (index differs from HEAD) each with a derived change type.
///
/// A staged path absent from the HEAD tree is **Added**; a staged path missing
/// from the working directory is **Deleted**; otherwise **Modified**.
pub fn get_staged_files_with_change_type(dir: impl AsRef<Path>) -> Result<Vec<ChangedFileStatus>> {
    let dir = dir.as_ref();
    let repo = open_repo(dir)?;
    get_staged_files_with_change_type_with_repo(&repo)
}

/// BUG-189 R1: staged change-type derivation against an ALREADY OPEN
/// repository handle — the public dir-based fn is a thin wrapper (open +
/// delegate) for other callers.
pub fn get_staged_files_with_change_type_with_repo(
    repo: &gix::Repository,
) -> Result<Vec<ChangedFileStatus>> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    // Single pass: staged detection + in-HEAD verdict in one loop.
    let verdicts = staged_files_with_repo(repo)?;
    Ok(staged_change_types(verdicts, workdir))
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
    get_unstaged_files_with_change_type_with_repo(&repo)
}

/// BUG-189 R1: unstaged change-type derivation against an ALREADY OPEN
/// repository handle (thin wrapper delegate target for the public fn).
pub fn get_unstaged_files_with_change_type_with_repo(
    repo: &gix::Repository,
) -> Result<Vec<ChangedFileStatus>> {
    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    // Stat-first: only paths whose index stat no longer matches (or whose
    // content hash then verifies as different) are reported (Modified).
    // Re-scan the index for tracked paths that vanished (Deleted).
    let mut out: Vec<ChangedFileStatus> = unstaged_files_with_repo(repo)?
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
