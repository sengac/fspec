//! Git status operations using gitoxide

use crate::error::{GitError, Result};
use crate::open_repo;
use gix::bstr::BStr;
use std::path::Path;

/// Get list of staged files (files added to the index)
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of file paths (relative to repository root) that are staged
pub fn get_staged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let repo = open_repo(dir.as_ref())?;
    let mut staged = Vec::new();

    // Get the index (staging area)
    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;

    // Get HEAD tree for comparison - use mutable tree for peel_to_entry_by_path
    let mut head_tree = match repo.head_commit() {
        Ok(commit) => Some(commit.tree().map_err(|e| GitError::Head(e.to_string()))?),
        Err(_) => None, // No commits yet
    };

    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = path_to_string(path);

        // Check if file differs from HEAD
        let is_staged = match &mut head_tree {
            Some(tree) => {
                // Use lookup_entry_by_path to properly traverse nested directories
                // e.g. "spec/features/test.feature" needs to traverse spec -> features -> test.feature
                match tree.lookup_entry_by_path(&path_str) {
                    Ok(Some(tree_entry)) => entry.id != tree_entry.id(),
                    Ok(None) => true, // New file (not in HEAD)
                    Err(_) => true,   // Error looking up, assume new file
                }
            }
            None => true, // No HEAD commit, all indexed files are staged
        };

        if is_staged {
            staged.push(path_str);
        }
    }

    Ok(staged)
}

/// Get list of unstaged files (modified files not yet staged)
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of file paths that have unstaged modifications
pub fn get_unstaged_files(dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let repo = open_repo(dir.as_ref())?;
    let mut unstaged = Vec::new();

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    let index = repo.index().map_err(|e| GitError::Status(e.to_string()))?;

    for entry in index.entries() {
        let path = entry.path(&index);
        let path_str = path_to_string(path);
        let full_path = workdir.join(&path_str);

        // Check if file exists and has been modified
        if full_path.exists() {
            // Read file and compare content hash
            let content = std::fs::read(&full_path)?;
            let hash =
                gix::objs::compute_hash(repo.object_hash(), gix::object::Kind::Blob, &content);

            // compute_hash returns Result, unwrap and compare
            if let Ok(computed_hash) = hash {
                if computed_hash != entry.id {
                    unstaged.push(path_str);
                }
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
/// * `dir` - Path to the repository root
///
/// # Returns
/// Branch name, or None if in detached HEAD state
pub fn get_current_branch(dir: impl AsRef<Path>) -> Result<Option<String>> {
    let repo = open_repo(dir.as_ref())?;

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
