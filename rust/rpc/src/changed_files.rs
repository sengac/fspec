//! RPC-355: build the combined changed-files list for the TUI transport.
//!
//! Ordering mirrors the TS `ChangedFilesViewer`: staged entries first, then
//! unstaged modifications/deletions, then untracked files (always Added).
//! BUG-657: delegates to the single shared-handle capture
//! (`codelet_git::status::capture_changed_files`, the one the git-state
//! watcher runs), which reads each HEAD tree once — no git logic is
//! reimplemented here.

use std::path::Path;

use codelet_git::status::capture_changed_files;
use codelet_rpc_types::ChangedFile;

/// Collect staged + unstaged + untracked changed files for `cwd`.
///
/// Returns `Err` when the underlying git inspection fails (BUG-465: the
/// caller returns it, never an empty list).
pub(crate) fn collect_changed_files(
    cwd: impl AsRef<Path>,
) -> codelet_git::Result<Vec<ChangedFile>> {
    Ok(capture_changed_files(cwd)?
        .into_iter()
        .map(|entry| ChangedFile {
            path: entry.path,
            change_type: entry.change_type,
            staged: entry.staged,
        })
        .collect())
}
