//! Git status operations using gitoxide

use crate::error::{GitError, Result};
use crate::head_tree::HeadEntries;
use crate::{discover_repo, open_repo};
use gix::bstr::BStr;
use std::collections::HashSet;
use std::path::Path;

// RPC-355: change-type derivation helpers live in a sibling module but are
// re-exported here so callers use the single `codelet_git::status::*` path.
// BUG-189: the single-pass combined capture lives in `crate::capture` and is
// re-exported here too — `codelet_git::status::*` remains the one door.
pub use crate::capture::{capture_changed_files, capture_changed_files_with, ChangedFileEntry};
pub use crate::change_type::{
    get_staged_files_with_change_type, get_unstaged_files_with_change_type, ChangeType,
    ChangedFileStatus,
};
pub use crate::head_tree::ObjectSource;

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
/// repository handle, reading HEAD's trees from the repository itself.
pub(crate) fn staged_files_with_repo(repo: &gix::Repository) -> Result<Vec<StagedFileVerdict>> {
    staged_files_with(repo, repo)
}

/// The staged pass, reading every HEAD tree through `source`.
///
/// BUG-657: the HEAD tree is walked ONCE (each tree object read exactly
/// once) into a path map, and every index entry is checked against that
/// map; no index path is resolved from the root tree. The in-HEAD verdict
/// is carried out alongside the staged list, so change-type derivation
/// never looks a path up again (BUG-189 R2).
///
/// Staged DELETIONS (a HEAD path removed from the index, e.g. `rm file &&
/// git add file`) have no index entry; they are the HEAD entries absent
/// from the index, listed after the index-side entries in HEAD-tree order.
/// Gitlinks (submodule entries) are compared but never listed as
/// deletions (mirrors `collect_worktree_files`).
pub(crate) fn staged_files_with(
    repo: &gix::Repository,
    source: &dyn ObjectSource,
) -> Result<Vec<StagedFileVerdict>> {
    // BUG-637: the staged pass reads a missing index file as an empty index.
    let index = crate::index_guard::index_or_empty(repo)?;
    let head = match head_tree_id(repo)? {
        Some(root) => Some(HeadEntries::read(source, &root)?),
        None => None, // Unborn branch: every indexed file is staged.
    };

    let mut staged = Vec::new();
    // All paths present in the index — needed to detect staged deletions
    // (HEAD paths ABSENT from the index).
    let mut index_paths: HashSet<String> = HashSet::new();
    for entry in index.entries() {
        let path = path_to_string(entry.path(&index));
        index_paths.insert(path.clone());
        let head_entry = head.as_ref().and_then(|head| head.get(&path));
        let verdict = match head_entry {
            Some(head_entry) if entry.id != head_entry.id => Some(StagedFileVerdict {
                in_head: true,
                // A null index OID on a path that exists in HEAD means
                // the file was staged as deleted (removed from the index).
                deleted_from_index: entry.id.is_null(),
                path,
            }),
            Some(_) => None, // Same object as HEAD: not staged.
            None => Some(StagedFileVerdict {
                in_head: false,
                deleted_from_index: false,
                path,
            }),
        };
        staged.extend(verdict);
    }

    if let Some(head) = &head {
        for head_entry in head.iter() {
            if !head_entry.gitlink && !index_paths.contains(&head_entry.path) {
                staged.push(StagedFileVerdict {
                    path: head_entry.path.clone(),
                    in_head: true,
                    deleted_from_index: true,
                });
            }
        }
    }

    Ok(staged)
}

/// The tree of the HEAD commit; `None` when the branch is unborn.
fn head_tree_id(repo: &gix::Repository) -> Result<Option<gix::ObjectId>> {
    let head = repo.head().map_err(|e| GitError::Head(e.to_string()))?;
    if head.is_unborn() {
        return Ok(None);
    }
    let commit = repo
        .head_commit()
        .map_err(|e| GitError::Head(e.to_string()))?;
    let tree = commit
        .tree_id()
        .map_err(|e| GitError::Head(e.to_string()))?;
    Ok(Some(tree.detach()))
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

    // BUG-637: the unstaged pass reads a missing index file as an empty index.
    let index = crate::index_guard::index_or_empty(repo)?;

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
/// handle (the shared handle keeps the index loaded and the ODB cache warm).
///
/// BUG-592: a working tree with no index file reads as an empty index, so
/// every file the ignore rules do not exclude is untracked. The walk never
/// enters a directory the ignore rules exclude: as in git, nothing under an
/// excluded directory can be re-included, so pruning it changes no result,
/// only the cost. A path the walk cannot read is an error naming it.
pub(crate) fn untracked_files_with_repo(repo: &gix::Repository) -> Result<Vec<String>> {
    let mut untracked = Vec::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    let index = crate::index_guard::index_or_empty(repo)?;

    // Get the excludes stack for proper gitignore checking
    let mut excludes = repo
        .excludes(
            &index,
            None, // No overrides
            gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )
        .map_err(|e| GitError::Other(format!("Failed to load excludes: {}", e)))?;

    let mut walk = walkdir::WalkDir::new(workdir).into_iter();
    while let Some(entry) = walk.next() {
        let entry = match entry {
            Ok(entry) => entry,
            // A path removed between being listed and being read is no
            // longer in the working tree, so it has nothing to list.
            Err(e)
                if e.io_error().map(std::io::Error::kind) == Some(std::io::ErrorKind::NotFound) =>
            {
                continue
            }
            Err(e) => {
                return Err(GitError::Status(format!(
                    "cannot walk the working tree: {e}"
                )))
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let rel_path = entry
            .path()
            .strip_prefix(workdir)
            .map_err(|e| GitError::Other(e.to_string()))?;

        if is_git_dir(&entry) {
            if entry.file_type().is_dir() {
                walk.skip_current_dir();
            }
            continue;
        }
        if entry.file_type().is_dir() {
            if is_excluded(&mut excludes, rel_path, gix::index::entry::Mode::DIR)? {
                walk.skip_current_dir();
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        // Check if file is in index using gix's path conversion
        let bstr_path = gix::path::into_bstr(rel_path);
        if index.entry_index_by_path(&bstr_path).is_ok() {
            continue;
        }
        if !is_excluded(&mut excludes, rel_path, gix::index::entry::Mode::FILE)? {
            // Convert to forward slashes for git path format
            untracked.push(rel_path.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(untracked)
}

/// Whether the repository's ignore rules exclude `rel_path` (checked as a
/// file or a directory per `mode`).
fn is_excluded(
    excludes: &mut gix::AttributeStack<'_>,
    rel_path: &Path,
    mode: gix::index::entry::Mode,
) -> Result<bool> {
    excludes
        .at_path(rel_path, Some(mode))
        .map(|platform| platform.is_excluded())
        .map_err(|e| {
            GitError::Status(format!(
                "cannot read the ignore rules for '{}': {e}",
                rel_path.display()
            ))
        })
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
