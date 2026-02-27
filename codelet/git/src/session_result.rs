//! Session result collection and application operations
//!
//! Provides operations for collecting diffs from session worktrees and
//! applying changes back to the main worktree.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::three_way_merge::write_conflict_markers;
use crate::tree_utils::{collect_worktree_files, get_tree_files};
use crate::utils::is_binary_content;
use crate::worktree::{remove_worktree, FSPEC_WORKTREES_DIR};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
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

    // Get the base commit tree
    let base_tree_files = get_tree_files(&repo, &base_commit)?;

    // Get current worktree files
    let worktree_files = collect_worktree_files(&worktree_path)?;

    // Compute differences
    let mut files_changed = Vec::new();
    let mut files_added = Vec::new();
    let mut files_deleted = Vec::new();
    let mut diff_parts = Vec::new();

    // Find modified and deleted files
    for (path, base_content) in &base_tree_files {
        if let Some(worktree_content) = worktree_files.get(path) {
            if base_content != worktree_content {
                files_changed.push(path.clone());
                let file_diff = generate_file_diff(path, base_content, worktree_content);
                diff_parts.push(file_diff);
            }
        } else {
            files_deleted.push(path.clone());
            let file_diff = generate_delete_diff(path, base_content);
            diff_parts.push(file_diff);
        }
    }

    // Find added files
    for (path, worktree_content) in &worktree_files {
        if !base_tree_files.contains_key(path) {
            files_added.push(path.clone());
            let file_diff = generate_add_diff(path, worktree_content);
            diff_parts.push(file_diff);
        }
    }

    // Sort for deterministic output
    files_changed.sort();
    files_added.sort();
    files_deleted.sort();

    let diff = diff_parts.join("\n");

    Ok(SessionResult {
        session_id: session_id.to_string(),
        diff,
        files_changed,
        files_added,
        files_deleted,
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
/// Ok(()) on success, error if conflicts detected or worktree not found
pub fn apply_session_changes(repo_path: impl AsRef<Path>, session_id: &str) -> Result<()> {
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

        // Apply resolved worktree content directly to main
        apply_worktree_to_main(&base_tree_files, &resolved_worktree_files, &main_workdir)?;
    } else {
        // FIRST-MERGE PATH: No pending state → run normal conflict detection.
        let potential_conflicts =
            detect_conflicts(&base_tree_files, &worktree_files, &main_files);

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
            apply_worktree_to_main(&base_tree_files, &merged_worktree_files, &main_workdir)?;
        } else {
            apply_worktree_to_main(&base_tree_files, &worktree_files, &main_workdir)?;
        }
    }

    // Remove session worktree
    remove_worktree(repo_path, session_id)?;

    Ok(())
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
    content
        .windows(8)
        .any(|w| w == b"<<<<<<< ")
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
        serde_json::to_string_pretty(&value)
            .map_err(|e| GitError::Other(format!("Failed to serialize pending conflicts: {}", e)))?,
    )?;
    Ok(())
}

/// Apply worktree changes to the main working directory.
///
/// Copies modified/added files from the worktree and removes deleted files.
fn apply_worktree_to_main(
    base_tree_files: &HashMap<String, Vec<u8>>,
    worktree_files: &HashMap<String, Vec<u8>>,
    main_workdir: &Path,
) -> Result<()> {
    // Copy modified/added files
    for (path, worktree_content) in worktree_files {
        let base_content = base_tree_files.get(path);
        let is_changed = base_content.map(|b| b != worktree_content).unwrap_or(true);

        if is_changed {
            let dest_path = main_workdir.join(path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest_path, worktree_content)?;
        }
    }

    // Remove deleted files
    for path in base_tree_files.keys() {
        if !worktree_files.contains_key(path) {
            let dest_path = main_workdir.join(path);
            if dest_path.exists() {
                fs::remove_file(&dest_path)?;
            }
        }
    }

    Ok(())
}

/// Detect conflicts between session and main worktree changes
fn detect_conflicts(
    base_tree_files: &HashMap<String, Vec<u8>>,
    worktree_files: &HashMap<String, Vec<u8>>,
    main_files: &HashMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut conflicts = Vec::new();

    // Check for files modified in both session and main since base_commit
    for (path, base_content) in base_tree_files {
        let session_changed = worktree_files
            .get(path)
            .map(|c| c != base_content)
            .unwrap_or(true); // deleted counts as changed

        let main_changed = main_files
            .get(path)
            .map(|c| c != base_content)
            .unwrap_or(false);

        if session_changed && main_changed {
            conflicts.push(path.clone());
        }
    }

    // Check for added files that exist in main with different content
    for path in worktree_files.keys() {
        if !base_tree_files.contains_key(path) && main_files.contains_key(path) {
            let session_content = worktree_files.get(path);
            let main_content = main_files.get(path);
            if session_content != main_content {
                conflicts.push(path.clone());
            }
        }
    }

    conflicts.sort();
    conflicts
}

// =============================================================================
// Diff generation helpers
// =============================================================================

/// Generate unified diff for a modified file
fn generate_file_diff(path: &str, old_content: &[u8], new_content: &[u8]) -> String {
    if is_binary_content(old_content) || is_binary_content(new_content) {
        return format!("Binary file {} changed\n", path);
    }

    let old_str = String::from_utf8_lossy(old_content);
    let new_str = String::from_utf8_lossy(new_content);
    let diff = TextDiff::from_lines(old_str.as_ref(), new_str.as_ref());

    let mut lines = vec![format!("--- a/{}", path), format!("+++ b/{}", path)];

    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        lines.push(format!(
            "{}{}",
            prefix,
            change.value().trim_end_matches('\n')
        ));
    }

    lines.join("\n")
}

/// Generate diff for a deleted file
fn generate_delete_diff(path: &str, content: &[u8]) -> String {
    if is_binary_content(content) {
        return format!("Binary file {} deleted\n", path);
    }

    let content_str = String::from_utf8_lossy(content);
    let mut lines = vec![format!("--- a/{}", path), "+++ /dev/null".to_string()];

    for line in content_str.lines() {
        lines.push(format!("-{}", line));
    }

    lines.join("\n")
}

/// Generate diff for an added file
fn generate_add_diff(path: &str, content: &[u8]) -> String {
    if is_binary_content(content) {
        return format!("Binary file {} added\n", path);
    }

    let content_str = String::from_utf8_lossy(content);
    let mut lines = vec!["--- /dev/null".to_string(), format!("+++ b/{}", path)];

    for line in content_str.lines() {
        lines.push(format!("+{}", line));
    }

    lines.join("\n")
}
