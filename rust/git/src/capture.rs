//! BUG-189 R1: the single-pass combined changed-files capture.
//!
//! A `GitState` capture must open the repository EXACTLY ONCE and share that
//! handle across the staged, unstaged, and untracked collectors — the three
//! separate public dir-based collectors each opened their own cold handle,
//! forcing the HEAD-tree objects to be re-decoded from pack three times per
//! capture (the 52% zlib-inflate hot spot). This module is the one door that
//! opens the repo once and reuses the warm handle end-to-end.
//!
//! The public dir-based collectors (`get_staged_files_with_change_type`,
//! `get_unstaged_files_with_change_type`, `get_untracked_files`) stay thin
//! wrappers for other callers (RPC, checkpoint restore, napi); they each open
//! their own handle — acceptable for a one-shot query, but the recurring
//! watcher capture goes through here.

use crate::change_type::{
    get_staged_files_with_change_type_with_repo, get_unstaged_files_with_change_type_with_repo,
};
use crate::error::Result;
use crate::status::untracked_files_with_repo;
use std::path::Path;

/// One combined changed-file entry: a repo-relative path, its single-letter
/// change type (`A`/`M`/`D`), and whether it is staged.
///
/// This is the git-layer analogue of `codelet_rpc_types::ChangedFile` — kept
/// in `codelet-git` so the capture can live entirely in the git crate
/// (no RPC-type dependency). The TUI/RPC boundary maps these to the wire
/// type with byte-identical field values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFileEntry {
    /// Repo-relative path.
    pub path: String,
    /// Single-letter change type (`A`dded / `M`odified / `D`eleted).
    pub change_type: String,
    /// `true` for staged entries, `false` for unstaged + untracked.
    pub staged: bool,
}

/// BUG-189 R1: capture the combined changed-files list (staged first, then
/// unstaged, then untracked — byte-identical ordering to the three separate
/// public collectors) using ONE shared `gix::Repository` handle.
///
/// # Ordering
/// Staged entries (in index order), then unstaged modifications, then
/// unstaged deletions, then untracked files (always `A`) — exactly what the
/// three separate public dir-based calls would produce, concatenated.
pub fn capture_changed_files(dir: impl AsRef<Path>) -> Result<Vec<ChangedFileEntry>> {
    // The single cold open for the whole capture — the shared handle is
    // passed into every collector below so the ODB cache stays warm.
    let repo = crate::open_repo(dir.as_ref())?;

    // Staged first (in-HEAD verdict reused from the single pass — BUG-189 R2).
    let mut out: Vec<ChangedFileEntry> = Vec::new();
    for entry in get_staged_files_with_change_type_with_repo(&repo)? {
        out.push(ChangedFileEntry {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: true,
        });
    }

    // Then unstaged modifications / deletions (stat-first — BUG-189 R3).
    for entry in get_unstaged_files_with_change_type_with_repo(&repo)? {
        out.push(ChangedFileEntry {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: false,
        });
    }

    // Finally untracked files — always Added, never staged.
    for path in untracked_files_with_repo(&repo)? {
        out.push(ChangedFileEntry {
            path,
            change_type: "A".to_string(),
            staged: false,
        });
    }

    Ok(out)
}
