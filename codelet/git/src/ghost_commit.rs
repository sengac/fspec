//! Ghost commit operations for fspec checkpoints
//!
//! Ghost commits are detached commits that:
//! - Capture complete working tree state (staged, unstaged, untracked)
//! - Have no branch reference (invisible to git log)
//! - Preserve parent relationship to HEAD
//! - Can be restored to return to exact state
//!
//! This module uses pure gitoxide (gix) - NO git CLI commands.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::tree_utils::collect_worktree_files;
use gix::bstr::BString;
use gix::objs::WriteTo;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Prefix for fspec checkpoint refs
const CHECKPOINT_REF_PREFIX: &str = "refs/fspec-checkpoints";

/// Result of creating a ghost commit checkpoint
#[derive(Debug, Clone)]
pub struct GhostCheckpoint {
    /// SHA of the ghost commit
    pub sha: String,
    /// SHA of the parent commit (HEAD at creation time)
    pub parent_sha: String,
    /// List of files captured in the checkpoint
    pub files: Vec<String>,
}

/// Result of restoring a ghost commit checkpoint
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// Whether restore was successful
    pub success: bool,
    /// Files that were restored
    pub restored_files: Vec<String>,
    /// Files that were deleted (existed after checkpoint but not in it)
    pub deleted_files: Vec<String>,
}

/// Create a ghost commit capturing current working tree state
///
/// Uses temporary index to avoid disturbing user's staging area.
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier for ref namespace
/// * `checkpoint_name` - Name for the checkpoint
///
/// # Returns
/// GhostCheckpoint with SHA, parent SHA, and captured files
///
/// # Algorithm
/// 1. Open repository
/// 2. Collect all working tree files (staged, unstaged, untracked)
/// 3. Build tree from collected files
/// 4. Create commit with tree and HEAD as parent (no ref update to branches)
/// 5. Store ref at refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}
pub fn create_ghost_commit(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
) -> Result<GhostCheckpoint> {
    let repo = open_repo(dir)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    // Get HEAD commit as parent (may not exist for new repos)
    let (parent_sha, head_files) = match repo.head_commit() {
        Ok(commit) => {
            let sha = commit.id().to_string();
            let files = crate::tree_utils::get_tree_files(&repo, &sha).unwrap_or_default();
            (sha, files)
        }
        Err(_) => (String::new(), std::collections::HashMap::new()), // No commits yet
    };

    // Collect all files from working tree
    let files_map = collect_worktree_files(workdir)?;

    // Compute changed files (files that differ from HEAD)
    let mut changed_files: Vec<String> = Vec::new();

    // Files that exist in working tree - check if different from HEAD
    for (path, content) in &files_map {
        match head_files.get(path) {
            Some(head_content) => {
                // File exists in both - check if different
                if content != head_content {
                    changed_files.push(path.clone());
                }
            }
            None => {
                // File only exists in working tree (new file)
                changed_files.push(path.clone());
            }
        }
    }

    // Files that exist in HEAD but not in working tree (deletions)
    for path in head_files.keys() {
        if !files_map.contains_key(path) {
            changed_files.push(path.clone());
        }
    }

    // Build tree from working tree files (only files that exist)
    let tree_id = build_tree_from_files(&repo, &files_map)?;

    // Create commit object
    let commit_id = create_commit_object(&repo, tree_id, &parent_sha)?;

    // Store ref at refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}
    let ref_name = format!(
        "{}/{}/{}",
        CHECKPOINT_REF_PREFIX, work_unit_id, checkpoint_name
    );
    store_ref(&repo, &ref_name, &commit_id)?;

    Ok(GhostCheckpoint {
        sha: commit_id.to_string(),
        parent_sha,
        files: changed_files,
    })
}

/// Build a tree object from a map of files
fn build_tree_from_files(
    repo: &gix::Repository,
    files: &std::collections::HashMap<String, Vec<u8>>,
) -> Result<gix::ObjectId> {
    use std::collections::BTreeMap;

    // Handle empty tree case - all files deleted
    if files.is_empty() {
        // Create an empty tree
        let empty_tree = gix::objs::Tree { entries: vec![] };
        return write_tree(repo, &empty_tree);
    }

    // Group files by directory structure
    // Key: directory path, Value: vec of (filename, blob_id, is_executable)
    let mut dir_entries: BTreeMap<String, Vec<(String, gix::ObjectId, bool)>> = BTreeMap::new();

    // Ensure root directory exists
    dir_entries.insert(String::new(), Vec::new());

    // First, create blob objects for all files
    for (path, content) in files {
        let blob_id = write_blob(repo, content)?;

        // Split path into directory and filename
        let path_obj = std::path::Path::new(path);
        let parent = path_obj
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let filename = path_obj
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // Ensure all parent directories exist in dir_entries
        // This is important for files like "spec/work-units.json" where "spec" needs to be added
        let mut current_path = parent.clone();
        while !current_path.is_empty() {
            if !dir_entries.contains_key(&current_path) {
                dir_entries.insert(current_path.clone(), Vec::new());
            }
            // Move up to parent
            current_path = std::path::Path::new(&current_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
        }

        // Check if file is executable (Unix only)
        #[cfg(unix)]
        let is_executable = {
            use std::os::unix::fs::PermissionsExt;
            let full_path = repo.workdir().unwrap().join(path);
            full_path
                .metadata()
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        };
        #[cfg(not(unix))]
        let is_executable = false;

        dir_entries
            .entry(parent)
            .or_default()
            .push((filename, blob_id, is_executable));
    }

    // Build trees bottom-up (deepest directories first)
    let mut tree_ids: BTreeMap<String, gix::ObjectId> = BTreeMap::new();

    // Get all directories sorted by depth (deepest first)
    let mut dirs: Vec<String> = dir_entries.keys().cloned().collect();
    dirs.sort_by(|a, b| {
        let a_depth = if a.is_empty() {
            0
        } else {
            a.matches('/').count() + 1
        };
        let b_depth = if b.is_empty() {
            0
        } else {
            b.matches('/').count() + 1
        };
        b_depth.cmp(&a_depth) // Sort descending (deepest first)
    });

    for dir_path in dirs {
        let entries = dir_entries.get(&dir_path).cloned().unwrap_or_default();

        // Build tree entries for this directory
        let mut tree_entries: Vec<gix::objs::tree::Entry> = Vec::new();

        // Add blob entries
        for (filename, blob_id, is_executable) in &entries {
            let mode: gix::object::tree::EntryMode = if *is_executable {
                gix::object::tree::EntryKind::BlobExecutable.into()
            } else {
                gix::object::tree::EntryKind::Blob.into()
            };

            tree_entries.push(gix::objs::tree::Entry {
                mode,
                filename: BString::from(filename.as_str()),
                oid: *blob_id,
            });
        }

        // Add subtree entries (directories that are children of this one)
        for (subtree_path, subtree_id) in &tree_ids {
            let subtree_parent = std::path::Path::new(subtree_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if subtree_parent == dir_path {
                let subtree_name = std::path::Path::new(subtree_path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();

                let mode: gix::object::tree::EntryMode = gix::object::tree::EntryKind::Tree.into();
                tree_entries.push(gix::objs::tree::Entry {
                    mode,
                    filename: BString::from(subtree_name.as_str()),
                    oid: *subtree_id,
                });
            }
        }

        // Sort entries by name (git requires sorted trees)
        tree_entries.sort_by(|a, b| a.filename.cmp(&b.filename));

        // Write tree object
        let tree = gix::objs::Tree {
            entries: tree_entries,
        };
        let tree_id = write_tree(repo, &tree)?;
        tree_ids.insert(dir_path, tree_id);
    }

    // Return root tree (empty string key)
    tree_ids
        .get("")
        .copied()
        .ok_or_else(|| GitError::Other("Failed to build root tree".to_string()))
}

/// Write a blob object to the repository
fn write_blob(repo: &gix::Repository, content: &[u8]) -> Result<gix::ObjectId> {
    let blob_id = repo
        .write_blob(content)
        .map_err(|e| GitError::Other(format!("Failed to write blob: {}", e)))?;
    Ok(blob_id.into())
}

/// Write a tree object to the repository
fn write_tree(repo: &gix::Repository, tree: &gix::objs::Tree) -> Result<gix::ObjectId> {
    let mut buf = Vec::new();
    tree.write_to(&mut buf)
        .map_err(|e| GitError::Other(format!("Failed to serialize tree: {}", e)))?;

    let tree_id = repo
        .write_object(gix::objs::Object::Tree(tree.clone()))
        .map_err(|e| GitError::Other(format!("Failed to write tree: {}", e)))?;
    Ok(tree_id.into())
}

/// Create a commit object without updating any branch refs
fn create_commit_object(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    parent_sha: &str,
) -> Result<gix::ObjectId> {
    use gix::date::Time;
    use smallvec::SmallVec;

    let now = chrono::Utc::now();
    let timestamp = now.timestamp();
    let offset = 0i32; // UTC

    let time = Time::new(timestamp, offset);
    let signature = gix::actor::Signature {
        name: BString::from("fspec"),
        email: BString::from("fspec@local"),
        time,
    };

    let parents: SmallVec<[gix::ObjectId; 1]> = if parent_sha.is_empty() {
        SmallVec::new()
    } else {
        let parent_id = repo.rev_parse_single(parent_sha.as_bytes()).map_err(|_| {
            GitError::InvalidCommitRef {
                commit_ref: parent_sha.to_string(),
            }
        })?;
        SmallVec::from_buf([parent_id.into()])
    };

    let commit = gix::objs::Commit {
        tree: tree_id,
        parents,
        author: signature.clone(),
        committer: signature,
        encoding: None,
        message: BString::from("fspec checkpoint"),
        extra_headers: vec![],
    };

    let commit_id = repo
        .write_object(gix::objs::Object::Commit(commit))
        .map_err(|e| GitError::Other(format!("Failed to write commit: {}", e)))?;

    Ok(commit_id.into())
}

/// Store a reference pointing to the given commit
fn store_ref(repo: &gix::Repository, ref_name: &str, commit_id: &gix::ObjectId) -> Result<()> {
    use gix::refs::transaction::PreviousValue;

    let ref_name = gix::refs::FullName::try_from(ref_name.to_string())
        .map_err(|e| GitError::Other(format!("Invalid ref name: {}", e)))?;

    repo.reference(
        ref_name,
        *commit_id,
        PreviousValue::Any,
        "fspec: create checkpoint",
    )
    .map_err(|e| GitError::Other(format!("Failed to create ref: {}", e)))?;

    Ok(())
}

/// Restore working tree from ghost commit
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier
/// * `checkpoint_name` - Name of the checkpoint to restore
/// * `force` - If true, overwrite without conflict detection
///
/// # Returns
/// RestoreResult with success status and affected files
///
/// # Algorithm
/// 1. Resolve ref to get ghost commit SHA
/// 2. Read tree from ghost commit
/// 3. For each file in tree, write to working directory
/// 4. Delete files that exist in working directory but not in checkpoint tree
pub fn restore_ghost_commit(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
    _force: bool,
) -> Result<RestoreResult> {
    let repo = open_repo(dir)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    // Resolve ref to get ghost commit SHA
    let ref_name = format!(
        "{}/{}/{}",
        CHECKPOINT_REF_PREFIX, work_unit_id, checkpoint_name
    );
    let commit_id = resolve_ref(&repo, &ref_name)?;

    // Get tree from ghost commit
    let checkpoint_files = crate::tree_utils::get_tree_files(&repo, &commit_id.to_string())?;

    // Get current working tree files
    let current_files = collect_worktree_files(&workdir)?;

    // Track files that will be restored and deleted
    let mut restored_files = Vec::new();
    let mut deleted_files = Vec::new();

    // Build sets for comparison
    let checkpoint_paths: HashSet<&String> = checkpoint_files.keys().collect();
    let current_paths: HashSet<&String> = current_files.keys().collect();

    // Restore files from checkpoint
    for (path, content) in &checkpoint_files {
        let full_path = workdir.join(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)?;
        restored_files.push(path.clone());
    }

    // Delete files that exist in working tree but not in checkpoint
    for path in &current_paths {
        if !checkpoint_paths.contains(*path) {
            let full_path = workdir.join(*path);
            if full_path.exists() {
                fs::remove_file(&full_path)?;
                deleted_files.push((*path).clone());
            }
        }
    }

    // Clean up empty directories
    cleanup_empty_dirs(&workdir)?;

    Ok(RestoreResult {
        success: true,
        restored_files,
        deleted_files,
    })
}

/// Resolve a ref to its target commit ID
fn resolve_ref(repo: &gix::Repository, ref_name: &str) -> Result<gix::ObjectId> {
    let mut reference = repo
        .find_reference(ref_name)
        .map_err(|e| GitError::Other(format!("Ref not found '{}': {}", ref_name, e)))?;

    let id = reference
        .peel_to_id_in_place()
        .map_err(|e| GitError::Other(format!("Failed to peel ref: {}", e)))?;

    Ok(id.into())
}

/// Clean up empty directories after file deletion
fn cleanup_empty_dirs(dir: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir)
        .contents_first(true)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() && entry.path() != dir {
            // Try to remove directory - will fail if not empty, which is fine
            let _ = fs::remove_dir(entry.path());
        }
    }
    Ok(())
}

/// List all ghost commit checkpoints for a work unit
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier
///
/// # Returns
/// Vector of checkpoint names
pub fn list_ghost_checkpoints(dir: &Path, work_unit_id: &str) -> Result<Vec<String>> {
    let repo = open_repo(dir)?;
    let mut checkpoints = Vec::new();

    let prefix = format!("{}/{}/", CHECKPOINT_REF_PREFIX, work_unit_id);

    // Iterate over all references
    let refs = repo
        .references()
        .map_err(|e| GitError::Other(format!("Failed to get references: {}", e)))?;

    for reference in refs.all().map_err(|e| GitError::Other(e.to_string()))? {
        let reference = reference.map_err(|e| GitError::Other(e.to_string()))?;
        let name = reference.name().as_bstr().to_string();

        if name.starts_with(&prefix) {
            let checkpoint_name = name.strip_prefix(&prefix).unwrap_or(&name);
            checkpoints.push(checkpoint_name.to_string());
        }
    }

    Ok(checkpoints)
}

/// Delete a ghost commit checkpoint
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier
/// * `checkpoint_name` - Name of the checkpoint to delete
pub fn delete_ghost_checkpoint(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
) -> Result<()> {
    let repo = open_repo(dir)?;
    let ref_name = format!(
        "{}/{}/{}",
        CHECKPOINT_REF_PREFIX, work_unit_id, checkpoint_name
    );

    // Find and delete the reference
    let reference = repo.find_reference(&ref_name).map_err(|e| {
        GitError::Other(format!("Checkpoint not found '{}': {}", checkpoint_name, e))
    })?;

    reference
        .delete()
        .map_err(|e| GitError::Other(format!("Failed to delete checkpoint: {}", e)))?;

    Ok(())
}

/// Get files that changed between checkpoint and current working tree
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier
/// * `checkpoint_name` - Name of the checkpoint
///
/// # Returns
/// Vector of file paths that differ
pub fn get_checkpoint_diff_files(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
) -> Result<Vec<String>> {
    let repo = open_repo(dir)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    // Resolve ref to get ghost commit SHA
    let ref_name = format!(
        "{}/{}/{}",
        CHECKPOINT_REF_PREFIX, work_unit_id, checkpoint_name
    );
    let commit_id = resolve_ref(&repo, &ref_name)?;

    // Get tree from ghost commit
    let checkpoint_files = crate::tree_utils::get_tree_files(&repo, &commit_id.to_string())?;

    // Get current working tree files
    let current_files = collect_worktree_files(&workdir)?;

    let mut diff_files = Vec::new();

    // Find modified and deleted files
    for (path, checkpoint_content) in &checkpoint_files {
        match current_files.get(path) {
            Some(current_content) => {
                if checkpoint_content != current_content {
                    diff_files.push(path.clone());
                }
            }
            None => {
                // File deleted since checkpoint
                diff_files.push(path.clone());
            }
        }
    }

    // Find added files
    for path in current_files.keys() {
        if !checkpoint_files.contains_key(path) {
            diff_files.push(path.clone());
        }
    }

    diff_files.sort();
    Ok(diff_files)
}
