//! RPC-355: build the combined changed-files list for the TUI transport.
//!
//! Ordering mirrors the TS `ChangedFilesViewer`: staged entries first, then
//! unstaged modifications/deletions, then untracked files (always Added).
//! Delegates entirely to the `codelet_git` change-type helpers — no git logic
//! is reimplemented here.

use std::path::Path;

use codelet_git::{
    get_staged_files_with_change_type, get_unstaged_files_with_change_type, get_untracked_files,
};
use codelet_rpc_types::ChangedFile;

/// Collect staged + unstaged + untracked changed files for `cwd`.
///
/// Returns `Err` only when the underlying git inspection fails; the caller
/// (`FspecService::changed_files`) maps that to an empty Vec via
/// `unwrap_or_default()` so a non-repo cwd never panics.
pub(crate) fn collect_changed_files(
    cwd: impl AsRef<Path>,
) -> codelet_git::Result<Vec<ChangedFile>> {
    let cwd = cwd.as_ref();
    let mut out: Vec<ChangedFile> = Vec::new();

    // Staged first.
    for entry in get_staged_files_with_change_type(cwd)? {
        out.push(ChangedFile {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: true,
        });
    }

    // Then unstaged modifications / deletions.
    for entry in get_unstaged_files_with_change_type(cwd)? {
        out.push(ChangedFile {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: false,
        });
    }

    // Finally untracked files — always Added, never staged.
    for path in get_untracked_files(cwd)? {
        out.push(ChangedFile {
            path,
            change_type: "A".to_string(),
            staged: false,
        });
    }

    Ok(out)
}
