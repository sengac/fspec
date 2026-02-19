//! Checkout operations for worktrees
//!
//! Provides utilities for checking out files from git trees to directories.

use crate::error::{GitError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Checkout files from a commit to the worktree directory
///
/// Walks the tree at the given commit and writes all files to the worktree.
///
/// # Arguments
/// * `repo` - Open git repository
/// * `worktree_path` - Target directory for checkout
/// * `commit_id` - Commit to checkout from
pub fn checkout_to_worktree(
    repo: &gix::Repository,
    worktree_path: &Path,
    commit_id: &gix::ObjectId,
) -> Result<()> {
    let commit = repo
        .find_object(*commit_id)
        .map_err(|e| GitError::Other(format!("Failed to find commit: {}", e)))?
        .into_commit();

    let tree_id = commit
        .tree_id()
        .map_err(|e| GitError::Other(format!("Failed to get tree id: {}", e)))?;

    let tree = repo
        .find_object(tree_id)
        .map_err(|e| GitError::Other(format!("Failed to find tree: {}", e)))?
        .into_tree();

    checkout_tree_recursive(repo, &tree, worktree_path, PathBuf::new())
}

/// Recursively checkout tree entries to the worktree
fn checkout_tree_recursive(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    worktree_path: &Path,
    relative_path: PathBuf,
) -> Result<()> {
    for entry in tree.iter() {
        let entry =
            entry.map_err(|e| GitError::Other(format!("Failed to read tree entry: {}", e)))?;
        let entry_path = relative_path.join(entry.filename().to_string());
        let full_path = worktree_path.join(&entry_path);

        match entry.mode().kind() {
            gix::object::tree::EntryKind::Tree => {
                fs::create_dir_all(&full_path)?;
                let subtree = repo
                    .find_object(entry.id())
                    .map_err(|e| GitError::Other(format!("Failed to find subtree: {}", e)))?
                    .into_tree();
                checkout_tree_recursive(repo, &subtree, worktree_path, entry_path)?;
            }
            gix::object::tree::EntryKind::Blob | gix::object::tree::EntryKind::BlobExecutable => {
                checkout_blob(repo, &entry, &full_path)?;
            }
            gix::object::tree::EntryKind::Link => {
                checkout_symlink(repo, &entry, &full_path)?;
            }
            gix::object::tree::EntryKind::Commit => {
                // Submodule - skip for now
            }
        }
    }

    Ok(())
}

/// Checkout a blob (file) entry
fn checkout_blob(
    repo: &gix::Repository,
    entry: &gix::object::tree::EntryRef<'_, '_>,
    full_path: &Path,
) -> Result<()> {
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let blob = repo
        .find_object(entry.id())
        .map_err(|e| GitError::Other(format!("Failed to find blob: {}", e)))?;
    let data: &[u8] = blob.data.as_ref();
    fs::write(full_path, data)?;

    // Set executable permission on Unix for executable blobs
    #[cfg(unix)]
    if entry.mode().kind() == gix::object::tree::EntryKind::BlobExecutable {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(full_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(full_path, perms)?;
    }

    Ok(())
}

/// Checkout a symbolic link entry
fn checkout_symlink(
    repo: &gix::Repository,
    entry: &gix::object::tree::EntryRef<'_, '_>,
    full_path: &Path,
) -> Result<()> {
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let blob = repo
        .find_object(entry.id())
        .map_err(|e| GitError::Other(format!("Failed to find link target: {}", e)))?;
    let target = String::from_utf8_lossy(blob.data.as_ref());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target.as_ref(), full_path)?;
    }

    #[cfg(windows)]
    {
        // On Windows, just write the link target as a file
        fs::write(full_path, target.as_ref())?;
    }

    Ok(())
}
