//! File collection utilities for worktrees and git trees
//!
//! Provides utilities for collecting file contents from directories and git trees.
//!
//! # IMPORTANT: Pure Gitoxide Implementation
//!
//! This module uses ONLY gitoxide (gix) - a pure Rust git implementation.
//! **ALL worktrees MUST have a properly initialized git index.**
//! There are NO fallbacks - if the index is missing, it's an error.
//!
//! # WT-006: rich tree snapshots
//!
//! Both collectors return a [`TreeSnapshot`] instead of a flat
//! `HashMap<String, Vec<u8>>` so that symlinks (by target), gitlinks
//! (submodule OIDs), and the executable bit survive the
//! diff/merge/checkpoint pipeline. Gitignored untracked files land in
//! the snapshot's `ignored` list — surfaced for honest reporting,
//! never part of the tracked-change set.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::tree_snapshot::TreeSnapshot;
use gix::index::entry::Mode;
use std::fs;
use std::path::Path;

/// Collect files from a worktree directory respecting .gitignore
///
/// This function collects:
/// - All tracked files (files in the git index), with kind and mode:
///   blobs (regular files), symlinks (by target), gitlinks (submodule OIDs)
/// - All untracked files that are NOT ignored by .gitignore
/// - All gitignored untracked files in `TreeSnapshot::ignored` (WT-006)
///
/// # IMPORTANT: Index Required
///
/// This function REQUIRES a properly initialized git index.
/// Worktrees created by fspec ALWAYS have an initialized index (GIT-035).
/// If the index is missing, this function returns an error.
///
/// # Arguments
/// * `worktree_path` - Path to the worktree root directory
///
/// # Returns
/// A [`TreeSnapshot`] mapping paths to contents, targets, and OIDs.
///
/// # Errors
/// Returns `GitError::CorruptedIndex` if the git index is missing or corrupted.
pub fn collect_worktree_files(worktree_path: &Path) -> Result<TreeSnapshot> {
    let repo = open_repo(worktree_path)?;
    let mut snap = TreeSnapshot::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    // Get the index - this MUST succeed for properly initialized repos/worktrees
    let index = repo.index().map_err(|e| GitError::CorruptedIndex {
        message: format!(
            "Git index not available at '{}'. This indicates a corrupted worktree \
                 or a repo without any commits. All fspec worktrees should have an \
                 initialized index. Ensure repo has at least one commit. Error: {}",
            worktree_path.display(),
            e
        ),
    })?;

    // 1. Collect all tracked entries from the index, mode-aware (WT-006)
    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = String::from_utf8_lossy(path).to_string();
        let full_path = workdir.join(&path_str);

        // Only include entries that still exist on disk (not deleted).
        // `symlink_metadata` distinguishes symlinks from the files they point at.
        let meta = match fs::symlink_metadata(&full_path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        let file_type = meta.file_type();

        match entry.mode {
            Mode::FILE | Mode::FILE_EXECUTABLE if file_type.is_file() => {
                let content = fs::read(&full_path)?;
                snap.blobs.insert(path_str.clone(), content);
                // WT-006: the on-disk executable bit is authoritative —
                // a chmod +x on a tracked 100644 file is a real mode
                // change that the flat index mode would miss.
                let is_exec = entry.mode == Mode::FILE_EXECUTABLE
                    || crate::tree_snapshot::is_file_executable(&full_path);
                if is_exec {
                    snap.executables.insert(path_str);
                }
            }
            Mode::SYMLINK if file_type.is_symlink() => {
                let target = fs::read_link(&full_path)?;
                snap.symlinks
                    .insert(path_str, target.to_string_lossy().to_string());
            }
            Mode::COMMIT => {
                // Gitlink (submodule): recorded atomically — never
                // descended into, never reported deleted (WT-006).
                snap.gitlinks.insert(path_str, entry.id);
            }
            _ => {}
        }
    }

    // 2. Collect untracked files (non-ignored → blobs/symlinks, ignored →
    //    the `ignored` list, WT-006). Use gitoxide's excludes stack for
    //    proper gitignore support.
    let excludes_result = repo.excludes(
        &index,
        None,
        gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
    );

    if let Ok(mut excludes) = excludes_result {
        // Walk the working directory
        for entry in walkdir::WalkDir::new(workdir)
            .into_iter()
            .filter_entry(|e| {
                !is_git_or_fspec_internal(e)
                    // WT-006: do not descend into nested git repositories
                    // (submodule working directories) — they are atomic
                    // gitlinks.
                    && (e.depth() == 0 || !is_nested_repo_dir(e.path()))
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let rel_path = entry
                .path()
                .strip_prefix(workdir)
                .map_err(|e| GitError::Other(e.to_string()))?;

            // Convert to forward slashes for git path format
            let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

            let file_type = entry.file_type();
            if !(file_type.is_file() || file_type.is_symlink()) {
                continue;
            }

            // Skip if already in our collection (tracked entry)
            let bstr_path = gix::path::into_bstr(rel_path);
            if index.entry_index_by_path(&bstr_path).is_ok() {
                continue;
            }

            // Check if file is ignored using proper gitignore support
            let is_ignored = excludes
                .at_path(rel_path, Some(gix::index::entry::Mode::FILE))
                .map(|platform| platform.is_excluded())
                .unwrap_or(false);

            if is_ignored {
                // WT-006: surface ignored files in the snapshot's `ignored`
                // list instead of dropping them silently.
                snap.ignored.push(rel_path_str);
                continue;
            }

            if file_type.is_symlink() {
                let target = fs::read_link(entry.path())?;
                snap.symlinks
                    .insert(rel_path_str, target.to_string_lossy().to_string());
            } else {
                let content = fs::read(entry.path())?;
                snap.blobs.insert(rel_path_str.clone(), content);
                if crate::tree_snapshot::is_file_executable(entry.path()) {
                    snap.executables.insert(rel_path_str);
                }
            }
        }
    }

    snap.ignored.sort();
    Ok(snap)
}

/// Check if entry is a git/fspec internal file or directory (should be skipped)
fn is_git_or_fspec_internal(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name();
    name == ".git" || name == ".fspec" || name == ".fspec-pending-conflicts"
}

/// Whether `dir` is the root of a (possibly nested) git repository —
/// i.e. it contains a `.git` entry of its own. Used to prune submodule
/// working directories from untracked-file walks (WT-006).
fn is_nested_repo_dir(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Get all files from a commit tree as a rich snapshot (WT-006)
///
/// # Arguments
/// * `repo` - Open git repository
/// * `commit_sha` - Commit SHA to read tree from
///
/// # Returns
/// A [`TreeSnapshot`] carrying blobs, symlink targets, gitlink OIDs,
/// and the executable bit for each path.
pub fn get_tree_files(repo: &gix::Repository, commit_sha: &str) -> Result<TreeSnapshot> {
    let commit_id =
        repo.rev_parse_single(commit_sha.as_bytes())
            .map_err(|_| GitError::InvalidCommitRef {
                commit_ref: commit_sha.to_string(),
            })?;

    let commit = repo
        .find_object(commit_id)
        .map_err(|e| GitError::Other(format!("Failed to find commit: {}", e)))?
        .into_commit();

    let tree_id = commit
        .tree_id()
        .map_err(|e| GitError::Other(format!("Failed to get tree id: {}", e)))?;

    let tree = repo
        .find_object(tree_id)
        .map_err(|e| GitError::Other(format!("Failed to find tree: {}", e)))?
        .into_tree();

    let mut snap = TreeSnapshot::new();
    collect_tree_snapshot_recursive(repo, &tree, "", &mut snap)?;

    Ok(snap)
}

/// Recursively collect files from a git tree into a [`TreeSnapshot`]
fn collect_tree_snapshot_recursive(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    prefix: &str,
    snap: &mut TreeSnapshot,
) -> Result<()> {
    for entry in tree.iter() {
        let entry =
            entry.map_err(|e| GitError::Other(format!("Failed to read tree entry: {}", e)))?;
        let entry_path = if prefix.is_empty() {
            entry.filename().to_string()
        } else {
            format!("{prefix}/{}", entry.filename())
        };

        match entry.mode().kind() {
            gix::object::tree::EntryKind::Tree => {
                let subtree = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find subtree: {}", e)))?
                    .into_tree();
                collect_tree_snapshot_recursive(repo, &subtree, &entry_path, snap)?;
            }
            gix::object::tree::EntryKind::Blob | gix::object::tree::EntryKind::BlobExecutable => {
                let blob = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find blob: {}", e)))?;
                snap.blobs.insert(entry_path.clone(), blob.data.to_vec());
                if entry.mode().kind() == gix::object::tree::EntryKind::BlobExecutable {
                    snap.executables.insert(entry_path);
                }
            }
            gix::object::tree::EntryKind::Link => {
                // Symlink: the blob stores the target path text (WT-006).
                let blob = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find link target: {}", e)))?;
                let target = String::from_utf8_lossy(blob.data.as_ref()).to_string();
                snap.symlinks.insert(entry_path, target);
            }
            gix::object::tree::EntryKind::Commit => {
                // Gitlink (submodule): recorded atomically by OID (WT-006).
                snap.gitlinks.insert(entry_path, entry.id().into());
            }
        }
    }

    Ok(())
}
