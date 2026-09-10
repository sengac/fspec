//! Session result collection and application operations
//!
//! Provides operations for collecting diffs from session worktrees and
//! applying changes back to the main worktree.
//!
//! WT-006: the diff/conflict/apply pipeline works on [`TreeSnapshot`]s
//! (blobs + symlinks + gitlinks + executable bit + the gitignored
//! `ignored` list) via [`crate::session_diff`] — gitignored files are
//! reported but never applied, symlinks round-trip by target, gitlinks
//! are atomic, and the executable bit is preserved.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::three_way_merge::write_conflict_markers;
use crate::tree_snapshot::TreeSnapshot;
use crate::tree_utils::{collect_worktree_files, get_tree_files};
use crate::worktree::{remove_worktree, FSPEC_WORKTREES_DIR};
use std::fs;
use std::path::Path;

/// Result of getting a session diff
///
/// Contains all information needed to review and apply session changes.
#[derive(Debug, Clone)]
pub struct SessionResult {
    /// Session ID this result belongs to
    pub session_id: String,
    /// Unified diff of all changes
    pub diff: String,
    /// List of files that were modified
    pub files_changed: Vec<String>,
    /// List of files that were added
    pub files_added: Vec<String>,
    /// List of files that were deleted
    pub files_deleted: Vec<String>,
    /// WT-006: gitignored untracked files present in the worktree.
    /// Surfaced for honest reporting ("nothing to merge" is about the
    /// tracked change set); never part of the unified diff, never
    /// merged, never counted toward `files_changed/added/deleted`.
    pub files_ignored: Vec<String>,
    /// The base commit the session was created from
    pub base_commit: String,
}

/// Get session diff comparing base commit to current worktree state
///
/// This compares the base_commit tree against the worktree's working directory,
/// capturing all changes including uncommitted modifications.
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// SessionResult with unified diff and file lists
pub fn get_session_diff(repo_path: impl AsRef<Path>, session_id: &str) -> Result<SessionResult> {
    let repo_path = repo_path.as_ref();
    let repo = open_repo(repo_path)?;

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    let git_dir = repo.git_dir();
    let worktree_git_dir = git_dir.join("worktrees").join(session_id);

    // Check if worktree exists
    if !worktree_path.exists() || !worktree_git_dir.exists() {
        return Err(GitError::WorktreeNotFound {
            session_id: session_id.to_string(),
        });
    }

    // Read base commit from worktree HEAD
    let head_path = worktree_git_dir.join("HEAD");
    let base_commit = fs::read_to_string(&head_path)?.trim().to_string();

    // Get the base commit tree (WT-006: rich snapshot with symlinks,
    // gitlinks, and the executable bit)
    let base_tree_files = get_tree_files(&repo, &base_commit)?;

    // Get current worktree files (WT-006: + gitignored `ignored` list)
    let worktree_files = collect_worktree_files(&worktree_path)?;

    // Compute differences (WT-006: snapshot-aware — symlinks by target,
    // gitlinks atomic, ignored files excluded from the tracked set)
    let (files_changed, files_added, files_deleted, diff) =
        crate::session_diff::compute_session_diff(&base_tree_files, &worktree_files);

    Ok(SessionResult {
        session_id: session_id.to_string(),
        diff,
        files_changed,
        files_added,
        files_deleted,
        files_ignored: worktree_files.ignored.clone(),
        base_commit,
    })
}

/// Apply session changes by copying files from session worktree to main worktree
///
/// This copies modified/added files and removes deleted files from the main worktree.
/// After successful application, the session worktree is removed.
///
/// When conflicts are detected (both session and main modified the same file),
/// a three-way merge is performed. Files that merge cleanly are applied; files
/// with overlapping changes get conflict markers written to the worktree and
/// a ConflictError is returned so the user can resolve them.
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// The post-apply worktree snapshot (the exact state that was copied
/// into main, post auto-merge / post resolution) on success; an error
/// if conflicts are detected or the worktree is not found.
pub fn apply_session_changes(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<TreeSnapshot> {
    let repo_path = repo_path.as_ref();
    let repo = open_repo(repo_path)?;

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    let git_dir = repo.git_dir();
    let worktree_git_dir = git_dir.join("worktrees").join(session_id);

    // Check if worktree exists
    if !worktree_path.exists() || !worktree_git_dir.exists() {
        return Err(GitError::WorktreeNotFound {
            session_id: session_id.to_string(),
        });
    }

    // Read base commit from worktree HEAD
    let head_path = worktree_git_dir.join("HEAD");
    let base_commit = fs::read_to_string(&head_path)?.trim().to_string();

    // Get the base commit tree
    let base_tree_files = get_tree_files(&repo, &base_commit)?;

    // Get current worktree files
    let worktree_files = collect_worktree_files(&worktree_path)?;

    // Get current main repo working directory state
    let main_workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?
        .to_path_buf();
    let main_files = collect_worktree_files(&main_workdir)?;

    // BUG-099: Check for pending conflict state BEFORE detect_conflicts().
    // This prevents the infinite loop where detect_conflicts() re-fires on
    // files the user has already resolved.
    if let Some(pending_files) = read_pending_conflicts(&worktree_path) {
        // RE-MERGE PATH: We have previously-conflicted files to check.
        let mut still_pending = Vec::new();

        for file in &pending_files {
            let file_path = worktree_path.join(file);
            if file_path.exists() {
                let content = fs::read(&file_path)?;
                if has_conflict_markers(&content) {
                    still_pending.push(file.clone());
                }
                // else: markers removed → resolved, will be applied as-is
            }
            // If file doesn't exist, treat as resolved (user deleted it)
        }

        if !still_pending.is_empty() {
            // Some files still have markers — tell LLM, DO NOT regenerate
            return Err(GitError::ConflictError {
                files: still_pending,
            });
        }

        // ALL conflicts resolved — delete state file and proceed.
        let state_path = worktree_path.join(PENDING_CONFLICTS_FILE);
        if state_path.exists() {
            fs::remove_file(&state_path)?;
        }

        // Re-read worktree files after deleting state file
        let resolved_worktree_files = collect_worktree_files(&worktree_path)?;

        // Apply resolved worktree content directly to main (WT-006:
        // snapshot-aware — symlinks, gitlinks, and exec bits preserved)
        crate::session_diff::apply_snapshot_to_main(
            &base_tree_files,
            &resolved_worktree_files,
            &main_workdir,
        )?;
    } else {
        // FIRST-MERGE PATH: No pending state → run normal conflict detection.
        let potential_conflicts =
            crate::session_diff::detect_conflicts(&base_tree_files, &worktree_files, &main_files);

        if !potential_conflicts.is_empty() {
            // BUG-098: Perform three-way merge and write conflict markers into
            // worktree files BEFORE returning ConflictError. This ensures the LLM
            // can actually read and resolve the conflict markers.
            let actual_conflicts = write_conflict_markers(
                &worktree_path,
                &potential_conflicts,
                &base_tree_files,
                &worktree_files,
                &main_files,
            )?;

            if !actual_conflicts.is_empty() {
                // BUG-099: Write state file BEFORE returning ConflictError.
                // This distinguishes 'first conflict detection' from 're-merge after
                // resolution' on the next call.
                write_pending_conflicts(&worktree_path, &actual_conflicts)?;

                return Err(GitError::ConflictError {
                    files: actual_conflicts,
                });
            }

            // All conflicts were auto-resolved by three-way merge.
            // Re-read worktree files since write_conflict_markers updated them
            // with auto-merged content, then fall through to apply.
            let merged_worktree_files = collect_worktree_files(&worktree_path)?;
            crate::session_diff::apply_snapshot_to_main(
                &base_tree_files,
                &merged_worktree_files,
                &main_workdir,
            )?;
        } else {
            crate::session_diff::apply_snapshot_to_main(
                &base_tree_files,
                &worktree_files,
                &main_workdir,
            )?;
        }
    }

    // WT-008: capture the post-apply worktree snapshot (what was just
    // copied into main, post auto-merge / post resolution) BEFORE the
    // worktree is deleted, so `commit_merged_session` can build the
    // merge commit tree from exactly the applied state.
    let applied = collect_worktree_files(&worktree_path)?;

    // Remove session worktree
    remove_worktree(repo_path, session_id)?;

    Ok(applied)
}

/// Abort a session by removing its worktree without applying changes
///
/// This is essentially an alias for remove_worktree, provided for semantic clarity.
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
pub fn abort_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<()> {
    remove_worktree(repo_path, session_id)
}

// =============================================================================
// Helper functions
// =============================================================================

/// State file name for pending conflict tracking (BUG-099)
const PENDING_CONFLICTS_FILE: &str = ".fspec-pending-conflicts";

/// Check if file content contains conflict markers
fn has_conflict_markers(content: &[u8]) -> bool {
    // Look for "<<<<<<< " at the start of a line
    content.windows(8).any(|w| w == b"<<<<<<< ")
}

/// Read pending conflicts state from worktree
///
/// Returns Some(file_list) if `.fspec-pending-conflicts` exists and is valid JSON,
/// None otherwise.
fn read_pending_conflicts(worktree_path: &Path) -> Option<Vec<String>> {
    let state_path = worktree_path.join(PENDING_CONFLICTS_FILE);
    if !state_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&state_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let files = value["files"]
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    Some(files)
}

/// Write pending conflicts state to worktree
///
/// Creates `.fspec-pending-conflicts` with a JSON object listing the conflicted files.
fn write_pending_conflicts(worktree_path: &Path, files: &[String]) -> Result<()> {
    let state_path = worktree_path.join(PENDING_CONFLICTS_FILE);
    let value = serde_json::json!({
        "files": files,
        "created_at": chrono::Utc::now().to_rfc3339()
    });
    fs::write(
        &state_path,
        serde_json::to_string_pretty(&value).map_err(|e| {
            GitError::Other(format!("Failed to serialize pending conflicts: {}", e))
        })?,
    )?;
    Ok(())
}
