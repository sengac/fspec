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
use crate::tree_snapshot::TreeSnapshot;
use crate::tree_utils::collect_worktree_files;
use codelet_rpc_types::CheckpointCounts;
use gix::bstr::BString;
use gix::objs::WriteTo;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Prefix for fspec checkpoint refs
const CHECKPOINT_REF_PREFIX: &str = "refs/fspec-checkpoints";

/// RPC-015: substring used to classify a checkpoint name as automatic.
///
/// Mirrors the TS `AUTO_CHECKPOINT_PATTERN` constant from
/// `src/utils/checkpoint-index.ts`. Names that contain `-auto-` (e.g.
/// `AUTH-001-auto-testing`) are produced by automatic state-transition
/// checkpoints; all other names are treated as user-created manual
/// checkpoints.
pub const AUTO_CHECKPOINT_PATTERN: &str = "-auto-";

/// RPC-015: aggregate `CheckpointCounts` across every ref under
/// `refs/fspec-checkpoints/`.
///
/// Iterates every reference in the repository, filters for those whose
/// name starts with `refs/fspec-checkpoints/`, and classifies the final
/// path segment (the checkpoint name) by [`AUTO_CHECKPOINT_PATTERN`].
///
/// Gracefully returns `Ok(CheckpointCounts::default())` for directories
/// that are not git repositories — matches the TS
/// `countCheckpoints(cwd)` ENOENT-tolerance contract.
///
/// # Arguments
/// * `dir` - Path to the repository root (or any directory)
///
/// # Returns
/// `Ok(CheckpointCounts { manual, auto })` on success, or a graceful
/// `Ok(CheckpointCounts::default())` when `dir` is not a git repository.
/// Returns `Err` only on unexpected gix errors (corrupt refs etc.).
pub fn count_checkpoints(dir: &Path) -> Result<CheckpointCounts> {
    let repo = match open_repo(dir) {
        Ok(r) => r,
        // Not a git repo — match TS countCheckpoints ENOENT behavior:
        // return zero counts rather than propagating the error.
        Err(_) => return Ok(CheckpointCounts::default()),
    };

    let refs = repo
        .references()
        .map_err(|e| GitError::Other(format!("Failed to get references: {}", e)))?;

    let prefix = format!("{}/", CHECKPOINT_REF_PREFIX);
    let mut manual: u32 = 0;
    let mut auto: u32 = 0;
    for reference in refs.all().map_err(|e| GitError::Other(e.to_string()))? {
        let reference = reference.map_err(|e| GitError::Other(e.to_string()))?;
        let name = reference.name().as_bstr().to_string();
        if !name.starts_with(&prefix) {
            continue;
        }
        // Extract the last path segment (the checkpoint name).
        let checkpoint_name = name.rsplit('/').next().unwrap_or(&name);
        if checkpoint_name.contains(AUTO_CHECKPOINT_PATTERN) {
            auto = auto.saturating_add(1);
        } else {
            manual = manual.saturating_add(1);
        }
    }
    Ok(CheckpointCounts { manual, auto })
}

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
        Err(_) => (String::new(), TreeSnapshot::new()), // No commits yet
    };

    // Collect all files from working tree (WT-006: rich snapshot)
    let files_map = collect_worktree_files(workdir)?;

    // Compute changed files (files that differ from HEAD). WT-006:
    // kind-aware — symlinks compare by target, gitlinks are atomic
    // (never reported as deleted when the worktree cannot clone them),
    // and the executable bit is part of blob identity.
    let mut changed_files: Vec<String> = Vec::new();

    // Base-side: modified or deleted (gitlinks skipped — atomic)
    for path in head_files.all_paths() {
        if head_files.gitlinks.contains_key(&path) {
            continue;
        }
        let head_content = head_files
            .entry_bytes(&path)
            .expect("path iterated from base snapshot");
        match files_map.entry_bytes(&path) {
            Some(work_content) => {
                if work_content != head_content {
                    changed_files.push(path);
                }
            }
            None => {
                // File deleted in working tree
                changed_files.push(path);
            }
        }
    }

    // Working-tree-side: added (new blobs/symlinks)
    for path in files_map.all_paths() {
        if head_files.contains_path(&path) {
            continue;
        }
        if files_map.entry_is_blob(&path) || files_map.symlinks.contains_key(&path) {
            changed_files.push(path);
        }
    }
    changed_files.sort();
    changed_files.dedup();

    // Build tree from working tree files (only files that exist)
    let tree_id = build_tree_from_snapshot(&repo, &files_map)?;

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

/// Build a tree object from a [`TreeSnapshot`] (WT-006).
///
/// Unlike the old byte-map builder, this preserves every git entry
/// kind:
/// - blobs → Blob / BlobExecutable (the executable bit from the
///   snapshot's `executables` set, captured from on-disk metadata)
/// - symlinks → Link entries whose blob is the target path text
/// - gitlinks → Commit entries carrying the submodule OID (atomic —
///   the checkpoint tree keeps them so restores never report the
///   submodule as deleted)
///
/// `ignored` paths are NOT part of the checkpoint tree: a checkpoint
/// captures git-representable state, and gitignored files are never
/// git state.
///
/// WT-008: `pub(crate)` so `merge_commit` reuses the same snapshot →
/// tree builder for the merge commit (one tree-builder, no drift).
pub(crate) fn build_tree_from_snapshot(repo: &gix::Repository, snap: &TreeSnapshot) -> Result<gix::ObjectId> {
    use std::collections::BTreeMap;

    // Handle empty tree case - all files deleted
    if snap.is_empty() {
        let empty_tree = gix::objs::Tree { entries: vec![] };
        return write_tree(repo, &empty_tree);
    }

    // (filename, object_id, entry_kind) per directory.
    let mut dir_entries: BTreeMap<
        String,
        Vec<(String, gix::ObjectId, gix::object::tree::EntryKind)>,
    > = BTreeMap::new();

    // Ensure root directory exists
    dir_entries.insert(String::new(), Vec::new());

    // Blobs: create blob objects, record with the executable bit
    for (path, content) in &snap.blobs {
        let blob_id = write_blob(repo, content)?;
        let kind = if snap.is_executable(path) {
            gix::object::tree::EntryKind::BlobExecutable
        } else {
            gix::object::tree::EntryKind::Blob
        };
        push_dir_entry(&mut dir_entries, path, blob_id, kind);
    }

    // Symlinks: Link entries whose blob stores the target text
    for (path, target) in &snap.symlinks {
        let blob_id = write_blob(repo, target.as_bytes())?;
        push_dir_entry(
            &mut dir_entries,
            path,
            blob_id,
            gix::object::tree::EntryKind::Link,
        );
    }

    // Gitlinks: Commit entries carrying the submodule OID (atomic)
    for (path, oid) in &snap.gitlinks {
        push_dir_entry(
            &mut dir_entries,
            path,
            *oid,
            gix::object::tree::EntryKind::Commit,
        );
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

        for (filename, oid, kind) in &entries {
            tree_entries.push(gix::objs::tree::Entry {
                mode: (*kind).into(),
                filename: BString::from(filename.as_str()),
                oid: *oid,
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

/// Push a (filename, oid, kind) entry into the directory map for
/// `path`, ensuring all parent directories are registered.
fn push_dir_entry(
    dir_entries: &mut std::collections::BTreeMap<
        String,
        Vec<(String, gix::ObjectId, gix::object::tree::EntryKind)>,
    >,
    path: &str,
    oid: gix::ObjectId,
    kind: gix::object::tree::EntryKind,
) {
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

    dir_entries
        .entry(parent)
        .or_default()
        .push((filename, oid, kind));
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
/// * `force` - If true, overwrite without conflict detection. If false and
///   the working tree diverges from the checkpoint (modified, deleted, or
///   added files — checkpoint tree vs working dir, NOT git status), the
///   function returns `GitError::RestoreConflict` listing the divergent
///   files WITHOUT writing or deleting anything (WT-010).
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
    force: bool,
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

    // WT-010: honor `force` — when false, detect the checkpoint-vs-workdir
    // divergence (modified / deleted-since-checkpoint /
    // added-after-checkpoint, the same comparison get_checkpoint_diff_files
    // uses) and bail out with a RestoreConflict BEFORE touching the tree.
    // Gitlinks are atomic (WT-006): never reported as divergent.
    let mut divergent: Vec<String> = Vec::new();
    if !force {
        let current_paths: HashSet<String> = current_files.all_paths().into_iter().collect();
        let checkpoint_blob_paths: HashSet<String> = checkpoint_files
            .blobs
            .keys()
            .cloned()
            .chain(checkpoint_files.symlinks.keys().cloned())
            .collect();
        for path in checkpoint_files.all_paths() {
            if checkpoint_files.gitlinks.contains_key(&path) {
                continue; // atomic
            }
            if !current_paths.contains(&path) {
                divergent.push(path);
                continue;
            }
            let checkpoint_bytes = checkpoint_files.entry_bytes(&path);
            let current_bytes = current_files.entry_bytes(&path);
            if checkpoint_bytes != current_bytes {
                divergent.push(path);
            }
        }
        for path in current_files.all_paths() {
            if !checkpoint_blob_paths.contains(&path) {
                divergent.push(path);
            }
        }
        if !divergent.is_empty() {
            divergent.sort();
            return Err(GitError::RestoreConflict { files: divergent });
        }
    }

    // Track files that will be restored and deleted
    let mut restored_files = Vec::new();
    let mut deleted_files = Vec::new();

    // Restore files from checkpoint (blobs with mode, symlinks; gitlinks
    // are atomic and skipped)
    for path in checkpoint_files.all_paths() {
        if checkpoint_files.gitlinks.contains_key(&path) {
            continue;
        }
        let full_path = workdir.join(&path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = checkpoint_files
            .entry_bytes(&path)
            .ok_or_else(|| GitError::Other(format!("missing entry for {path}")))?;
        if checkpoint_files.entry_is_blob(&path) {
            let executable = checkpoint_files.executables.contains(&path);
            crate::tree_snapshot::write_file_with_mode(&workdir, &path, &content, executable)?;
        } else {
            // symlink: entry_bytes is the target path
            let target = String::from_utf8_lossy(&content).to_string();
            if full_path.exists() {
                fs::remove_file(&full_path)?;
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&target, &full_path)?;
            }
            #[cfg(windows)]
            // Windows: materialize as a plain file holding the target path
            // (no native symlink support in the release profile).
            {
                fs::write(&full_path, &target)?;
            }
        }
        restored_files.push(path);
    }

    // Delete files that exist in working tree but not in checkpoint
    // (gitignored paths are never touched)
    let checkpoint_blob_paths: HashSet<&str> = checkpoint_files
        .blobs
        .keys()
        .chain(checkpoint_files.symlinks.keys())
        .map(String::as_str)
        .collect();
    for path in current_files.all_paths() {
        if checkpoint_blob_paths.contains(path.as_str()) {
            continue;
        }
        let full_path = workdir.join(&path);
        if full_path.exists() {
            fs::remove_file(&full_path)?;
            deleted_files.push(path);
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

/// Restore a single file from a checkpoint into the working tree (RPC-362).
///
/// Resolves the checkpoint ref, reads the file's content from the checkpoint
/// tree, and writes it to the working directory (creating parent dirs). When
/// the file does not exist in the checkpoint tree it is removed from the
/// working directory instead (restoring "this file was absent" state).
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `work_unit_id` - Work unit identifier
/// * `checkpoint_name` - Name of the checkpoint
/// * `path` - Repo-relative path of the file to restore
pub fn restore_ghost_commit_file(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
    path: &str,
) -> Result<()> {
    let repo = open_repo(dir)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();

    let ref_name = format!(
        "{}/{}/{}",
        CHECKPOINT_REF_PREFIX, work_unit_id, checkpoint_name
    );
    let commit_id = resolve_ref(&repo, &ref_name)?;
    let checkpoint_files = crate::tree_utils::get_tree_files(&repo, &commit_id.to_string())?;

    let full_path = workdir.join(path);
    match checkpoint_files.entry_bytes(path) {
        Some(content) => {
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if checkpoint_files.entry_is_blob(path) {
                let executable = checkpoint_files.executables.contains(path);
                crate::tree_snapshot::write_file_with_mode(&workdir, path, &content, executable)?;
            } else if checkpoint_files.symlinks.contains_key(path) {
                // symlink: content is the target path (atomic gitlinks are
                // skipped — they never carry entry_bytes)
                if full_path.exists() {
                    fs::remove_file(&full_path)?;
                }
                let target = String::from_utf8_lossy(&content).to_string();
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&target, &full_path)?;
                }
                #[cfg(windows)]
                // Windows: materialize as a plain file holding the target path.
                {
                    fs::write(&full_path, &target)?;
                }
            }
        }
        None => {
            // File absent in the checkpoint — restoring means deleting it.
            if full_path.exists() {
                fs::remove_file(&full_path)?;
            }
        }
    }

    Ok(())
}

/// Resolve a ref to its target commit ID (internal helper)
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

/// List every ghost-commit checkpoint across ALL work units (RPC-362).
///
/// Iterates every reference under `refs/fspec-checkpoints/` and splits the
/// suffix into `(work_unit_id, checkpoint_name)` on the first `/`. Returns the
/// pairs in arbitrary ref-iteration order (callers sort as needed). Gracefully
/// returns an empty Vec for directories that are not git repositories — matches
/// the ENOENT-tolerance of [`count_checkpoints`].
///
/// # Arguments
/// * `dir` - Path to the repository root
///
/// # Returns
/// Vector of `(work_unit_id, checkpoint_name)` pairs.
pub fn list_all_ghost_checkpoints(dir: &Path) -> Result<Vec<(String, String)>> {
    // TUI-109: two front doors, one source of truth — the non-streaming
    // entry point delegates to the streaming variant with a no-op
    // callback so the CLI output cannot drift from the wire path.
    list_all_ghost_checkpoints_stream(dir, &mut |_| {})
}

/// TUI-109: streaming variant of [`list_all_ghost_checkpoints`] —
/// invokes `on_item` once per checkpoint ref (in ref-iteration order)
/// and returns the same `Vec<(work_unit_id, checkpoint_name)>` pairs.
///
/// The git crate only *ticks*; the rpc crate *shapes* (limits, sort,
/// done flag) the progress frames it builds from these ticks.
pub fn list_all_ghost_checkpoints_stream(
    dir: &Path,
    on_item: &mut dyn FnMut(&(String, String)),
) -> Result<Vec<(String, String)>> {
    let repo = match open_repo(dir) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    let prefix = format!("{}/", CHECKPOINT_REF_PREFIX);
    let mut out: Vec<(String, String)> = Vec::new();

    let refs = repo
        .references()
        .map_err(|e| GitError::Other(format!("Failed to get references: {}", e)))?;

    for reference in refs.all().map_err(|e| GitError::Other(e.to_string()))? {
        let reference = reference.map_err(|e| GitError::Other(e.to_string()))?;
        let name = reference.name().as_bstr().to_string();

        if let Some(suffix) = name.strip_prefix(&prefix) {
            // suffix == "<work_unit_id>/<checkpoint_name>". Split on the first
            // '/' so checkpoint names containing slashes stay intact.
            if let Some((work_unit_id, checkpoint_name)) = suffix.split_once('/') {
                let pair = (work_unit_id.to_string(), checkpoint_name.to_string());
                on_item(&pair);
                out.push(pair);
            }
        }
    }

    Ok(out)
}
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

    // WT-006: snapshot-aware diff — symlinks compare by target, the
    // executable bit is implicit in byte comparison only for blobs, and
    // gitlinks are atomic (never diffed).
    let (files_changed, files_added, files_deleted, _diff) =
        crate::session_diff::compute_session_diff(&checkpoint_files, &current_files);

    let mut diff_files = files_changed;
    diff_files.extend(files_added);
    diff_files.extend(files_deleted);
    diff_files.sort();
    diff_files.dedup();
    Ok(diff_files)
}
