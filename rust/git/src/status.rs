//! Git status operations using gitoxide

use crate::error::{GitError, Result};
use crate::{discover_repo, open_repo};
use gix::bstr::BStr;
use std::path::Path;

// RPC-355: change-type derivation helpers live in a sibling module but are
// re-exported here so callers use the single `codelet_git::status::*` path.
// BUG-189: the single-pass combined capture lives in `crate::capture` and is
// re-exported here too — `codelet_git::status::*` remains the one door.
pub use crate::capture::{capture_changed_files, ChangedFileEntry};
pub use crate::change_type::{
    get_staged_files_with_change_type, get_unstaged_files_with_change_type, ChangeType,
    ChangedFileStatus,
};

/// One index entry that differs from HEAD (a staged path), together with
/// the per-path verdicts derived in the SAME pass (BUG-189 R2): whether the
/// path exists in the HEAD tree at all, and whether it was REMOVED from the
/// index (staged deletion) — so change-type derivation reuses the verdicts
/// instead of re-deriving them with a second tree lookup.
#[derive(Debug, Clone)]
pub(crate) struct StagedFileVerdict {
    /// Repo-relative path.
    pub path: String,
    /// The path exists in the HEAD tree (false for newly staged files).
    pub in_head: bool,
    /// The index entry carries a null OID while the path exists in HEAD —
    /// i.e. the file was staged as deleted (removed from the index).
    pub deleted_from_index: bool,
}

/// Get list of staged files (files added to the index)
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of file paths (relative to repository root) that are staged
pub fn get_staged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let repo = open_repo(dir.as_ref())?;
    staged_files_with_repo(&repo).map(|verdicts| verdicts.into_iter().map(|v| v.path).collect())
}

/// BUG-189 R1/R2: single-pass staged detection against an ALREADY OPEN
/// repository handle. The index is scanned once and each index entry is
/// compared against the HEAD tree in that same pass — exactly ONE tree
/// lookup per indexed path — and the in-HEAD verdict is carried out
/// alongside the staged list, so `get_staged_files_with_change_type`
/// never re-issues a second lookup per staged path.
///
/// Staged DELETIONS (a HEAD path removed from the index, e.g. `rm file &&
/// git add file`) have no index entry to look up, so they cannot be
/// discovered by the index-side pass. They are detected by enumerating
/// the HEAD tree ONCE (recursively, on the shared warm handle — the ODB
/// cache keeps the tree objects hot) and taking the complement of the
/// index paths: a HEAD path with no index entry is a staged deletion.
pub(crate) fn staged_files_with_repo(repo: &gix::Repository) -> Result<Vec<StagedFileVerdict>> {
    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;

    // Get HEAD tree for comparison - use mutable tree for peel_to_entry_by_path
    let mut head_tree = match repo.head_commit() {
        Ok(commit) => Some(commit.tree().map_err(|e| GitError::Head(e.to_string()))?),
        Err(_) => None, // No commits yet
    };

    let mut staged = Vec::new();
    // All paths present in the index — needed to detect staged deletions
    // (HEAD paths ABSENT from the index) after the single lookup pass.
    let mut index_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = path_to_string(path);
        index_paths.insert(path_str.clone());

        // Check if file differs from HEAD — the ONLY tree comparison issued
        // for this path. The in-HEAD verdict is computed here and reused
        // by change-type derivation (BUG-189 R2: no second pass).
        // The path lookup traverses nested directories properly, e.g.
        // "spec/features/test.feature" walks spec -> features.
        let (is_staged, in_head, deleted_from_index) = match &mut head_tree {
            Some(tree) => match tree.lookup_entry_by_path(&path_str) {
                Ok(Some(tree_entry)) => (
                    entry.id != tree_entry.id(),
                    true,
                    // A null index OID on a path that exists in HEAD means
                    // the file was staged as deleted (removed from the
                    // index) — carried out as a verdict so the change-type
                    // letter can be derived without another pass.
                    entry.id.as_slice().iter().all(|&b| b == 0),
                ),
                Ok(None) => (true, false, false), // New file (not in HEAD)
                Err(_) => (true, false, false),   // Error looking up, assume new file
            },
            None => (true, false, false), // No HEAD commit, all indexed files are staged
        };

        if is_staged {
            staged.push(StagedFileVerdict {
                path: path_str,
                in_head,
                deleted_from_index,
            });
        }
    }

    // Staged deletions: a path that exists in the HEAD tree but is ABSENT
    // from the index cannot be discovered by index-side lookups — it has no
    // index entry to look up. Enumerate the HEAD tree once (the shared
    // handle keeps every tree object warm in the ODB cache, so this is a
    // single linear pass, not re-decoded pack reads) and take the
    // complement of the index paths.
    if let Some(head_tree) = &head_tree {
        let mut head_paths: Vec<String> = Vec::new();
        collect_head_blob_paths(repo, head_tree, "", &mut head_paths)?;
        for path in head_paths {
            if !index_paths.contains(&path) {
                staged.push(StagedFileVerdict {
                    in_head: true,
                    deleted_from_index: true,
                    path,
                });
            }
        }
    }

    Ok(staged)
}

/// Collect every blob / symlink path under `tree` (recursing into
/// subtrees) into `out` as repo-relative paths. Gitlinks (submodule
/// entries) are skipped — they are atomic and out of scope for
/// file-level change detection (mirrors `collect_worktree_files`).
fn collect_head_blob_paths(
    repo: &gix::Repository,
    tree: &gix::Tree,
    prefix: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    for entry in tree.iter() {
        let entry = entry.map_err(|e| GitError::Head(e.to_string()))?;
        let path = if prefix.is_empty() {
            entry.filename().to_string()
        } else {
            format!("{prefix}/{}", entry.filename())
        };
        match entry.mode().kind() {
            gix::object::tree::EntryKind::Tree => {
                let subtree = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Head(e.to_string()))?
                    .into_tree();
                collect_head_blob_paths(repo, &subtree, &path, out)?;
            }
            gix::object::tree::EntryKind::Blob
            | gix::object::tree::EntryKind::BlobExecutable
            | gix::object::tree::EntryKind::Link => {
                out.push(path);
            }
            gix::object::tree::EntryKind::Commit => {} // Gitlink: skipped
        }
    }
    Ok(())
}

/// Get list of unstaged files (modified files not yet staged)
///
/// BUG-189 R3: stat-first — the on-disk mtime + size are compared against
/// the index entry's recorded stat BEFORE any content read. A stat match
/// means "not modified" (exactly what git does), so unchanged files cost a
/// single `stat` syscall instead of a full-file read + sha1. Only a stat
/// mismatch (or a zeroed/unknown index stat) falls back to reading the file
/// and comparing content hashes.
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of file paths that have unstaged modifications
pub fn get_unstaged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let repo = open_repo(dir.as_ref())?;
    unstaged_files_with_repo(&repo)
}

/// BUG-189 R1: stat-first unstaged detection against an ALREADY OPEN
/// repository handle (see [`get_unstaged_files`] for the stat-first
/// contract).
pub(crate) fn unstaged_files_with_repo(repo: &gix::Repository) -> Result<Vec<String>> {
    let mut unstaged = Vec::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;

    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = path_to_string(path);
        let full_path = workdir.join(&path_str);

        // BUG-189 R3: the quick stat check — mtime + size against the index
        // entry's recorded stat — must succeed (and the file must exist)
        // before any content read is even considered.
        let meta = match std::fs::metadata(&full_path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        let mtime = match meta.modified() {
            Ok(mtime) => mtime,
            Err(_) => continue, // Unknown mtime: treat as unchanged (same
                                // tolerance as the previous exists()-only gate)
        };
        let since_epoch = mtime
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let stat_matches = entry.stat.mtime.secs == since_epoch.as_secs() as u32
            && entry.stat.size == meta.len() as u32;
        if stat_matches {
            // The stat the index recorded still matches the file on disk —
            // "not modified", exactly what git trusts. No read, no hash.
            continue;
        }

        // Fallback / verification path: the stat (or the file's existence
        // pattern) no longer matches the index — read + hash to decide.
        let content = match std::fs::read(&full_path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        let hash = gix::objs::compute_hash(repo.object_hash(), gix::object::Kind::Blob, &content);
        if let Ok(computed_hash) = hash {
            if computed_hash != entry.id {
                unstaged.push(path_str);
            }
        }
    }

    Ok(unstaged)
}

/// Get list of untracked files (files not tracked by git)
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of file paths that are not tracked by git
pub fn get_untracked_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let repo = open_repo(dir.as_ref())?;
    untracked_files_with_repo(&repo)
}

/// BUG-189 R1: untracked-file walk against an ALREADY OPEN repository
/// handle (the shared handle keeps the index loaded and the ODB cache
/// warm; the walk itself is unchanged — see BUG-189 assumption 1 for why
/// gating it on worktree fs-events is out of scope for this unit).
pub(crate) fn untracked_files_with_repo(repo: &gix::Repository) -> Result<Vec<String>> {
    let mut untracked = Vec::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;

    // Get the excludes stack for proper gitignore checking
    let mut excludes = repo
        .excludes(
            &index,
            None, // No overrides
            gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )
        .map_err(|e| GitError::Other(format!("Failed to load excludes: {}", e)))?;

    // Walk the working directory
    for entry in walkdir::WalkDir::new(workdir)
        .into_iter()
        .filter_entry(|e| !is_git_dir(e))
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let rel_path = entry
                .path()
                .strip_prefix(workdir)
                .map_err(|e| GitError::Other(e.to_string()))?;

            // Convert to forward slashes for git path format
            let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

            // Check if file is in index using gix's path conversion
            let bstr_path = gix::path::into_bstr(rel_path);
            let is_in_index = index.entry_index_by_path(&bstr_path).is_ok();

            if !is_in_index {
                // Check if file is ignored using proper gitignore support
                let is_ignored = excludes
                    .at_path(rel_path, Some(gix::index::entry::Mode::FILE))
                    .map(|platform| platform.is_excluded())
                    .unwrap_or(false);

                if !is_ignored {
                    untracked.push(rel_path_str);
                }
            }
        }
    }

    Ok(untracked)
}

/// Get current branch name
///
/// # Arguments
/// * `dir` - Path to the repository root, or any subdirectory within a git repo.
///   Uses `gix::discover` to walk up parent directories to find the enclosing repo.
///
/// # Returns
/// Branch name, or None if in detached HEAD state
pub fn get_current_branch(dir: impl AsRef<Path>) -> Result<Option<String>> {
    let repo = discover_repo(dir.as_ref())?;

    let head = repo.head().map_err(|e| GitError::Head(e.to_string()))?;

    match head.kind {
        gix::head::Kind::Symbolic(reference) => {
            // Reference struct has name field which is FullName
            let name = reference.name.shorten().to_string();
            Ok(Some(name))
        }
        gix::head::Kind::Detached { .. } => Ok(None),
        gix::head::Kind::Unborn(reference) => {
            // FullName has shorten() method
            let name = reference.shorten().to_string();
            Ok(Some(name))
        }
    }
}

// Helper functions

fn path_to_string(path: &BStr) -> String {
    String::from_utf8_lossy(path).to_string()
}

fn is_git_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_name() == ".git"
}
