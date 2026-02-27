//! File collection utilities for worktrees and git trees
//!
//! Provides utilities for collecting file contents from directories and git trees.
//!
//! # IMPORTANT: Pure Gitoxide Implementation
//!
//! This module uses ONLY gitoxide (gix) - a pure Rust git implementation.
//! **ALL worktrees MUST have a properly initialized git index.**
//! There are NO fallbacks - if the index is missing, it's an error.

use crate::error::{GitError, Result};
use crate::open_repo;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Collect files from a worktree directory respecting .gitignore
///
/// This function collects:
/// - All tracked files (files in the git index)
/// - All untracked files that are NOT ignored by .gitignore
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
/// HashMap mapping relative paths to file contents
///
/// # Errors
/// Returns `GitError::CorruptedIndex` if the git index is missing or corrupted.
pub fn collect_worktree_files(worktree_path: &Path) -> Result<HashMap<String, Vec<u8>>> {
    let repo = open_repo(worktree_path)?;
    let mut files = HashMap::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    // Get the index - this MUST succeed for properly initialized repos/worktrees
    let index = repo.index().map_err(|e| {
        GitError::CorruptedIndex {
            message: format!(
                "Git index not available at '{}'. This indicates a corrupted worktree \
                 or a repo without any commits. All fspec worktrees should have an \
                 initialized index. Ensure repo has at least one commit. Error: {}",
                worktree_path.display(),
                e
            ),
        }
    })?;

    // 1. Collect all tracked files from the index
    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = String::from_utf8_lossy(path).to_string();
        let full_path = workdir.join(&path_str);

        // Only include if file exists in working tree (not deleted)
        if full_path.is_file() {
            let content = fs::read(&full_path)?;
            files.insert(path_str, content);
        }
    }

    // 2. Collect untracked files that are NOT ignored
    // Use gitoxide's excludes stack for proper gitignore support
    let excludes_result = repo.excludes(
        &index,
        None,
        gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
    );

    if let Ok(mut excludes) = excludes_result {
        // Walk the working directory
        for entry in walkdir::WalkDir::new(workdir)
            .into_iter()
            .filter_entry(|e| !is_git_or_fspec_internal(e))
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let rel_path = entry
                    .path()
                    .strip_prefix(workdir)
                    .map_err(|e| GitError::Other(e.to_string()))?;

                // Convert to forward slashes for git path format
                let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

                // Skip if already in our collection (tracked file)
                if files.contains_key(&rel_path_str) {
                    continue;
                }

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
                        let content = fs::read(entry.path())?;
                        files.insert(rel_path_str, content);
                    }
                }
            }
        }
    }

    Ok(files)
}

/// Check if entry is a git/fspec internal file or directory (should be skipped)
fn is_git_or_fspec_internal(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name();
    name == ".git" || name == ".fspec" || name == ".fspec-pending-conflicts"
}

/// Get all files from a commit tree as a map of path -> content
///
/// # Arguments
/// * `repo` - Open git repository
/// * `commit_sha` - Commit SHA to read tree from
///
/// # Returns
/// HashMap mapping relative paths to file contents
pub fn get_tree_files(
    repo: &gix::Repository,
    commit_sha: &str,
) -> Result<HashMap<String, Vec<u8>>> {
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

    let mut files = HashMap::new();
    collect_tree_files_recursive(repo, &tree, PathBuf::new(), &mut files)?;

    Ok(files)
}

/// Recursively collect files from a git tree
fn collect_tree_files_recursive(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    prefix: PathBuf,
    files: &mut HashMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in tree.iter() {
        let entry =
            entry.map_err(|e| GitError::Other(format!("Failed to read tree entry: {}", e)))?;
        let entry_path = prefix.join(entry.filename().to_string());

        match entry.mode().kind() {
            gix::object::tree::EntryKind::Tree => {
                let subtree = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find subtree: {}", e)))?
                    .into_tree();
                collect_tree_files_recursive(repo, &subtree, entry_path, files)?;
            }
            gix::object::tree::EntryKind::Blob | gix::object::tree::EntryKind::BlobExecutable => {
                let blob = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find blob: {}", e)))?;
                let path_str = entry_path.to_string_lossy().to_string();
                files.insert(path_str, blob.data.to_vec());
            }
            _ => {
                // Skip symlinks and submodules for simplicity
            }
        }
    }

    Ok(())
}
