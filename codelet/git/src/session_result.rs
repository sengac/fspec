//! Session result collection and application operations
//!
//! Provides operations for collecting diffs from session worktrees and
//! applying changes back to the main worktree.

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::tree_utils::{collect_worktree_files, get_tree_files};
use crate::utils::is_binary_content;
use crate::worktree::{remove_worktree, FSPEC_WORKTREES_DIR};
use similar::{ChangeTag, TextDiff};
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

    // Detect conflicts: files modified in both session and main since base_commit
    let conflicts = detect_conflicts(&base_tree_files, &worktree_files, &main_files);

    if !conflicts.is_empty() {
        return Err(GitError::ConflictError { files: conflicts });
    }

    // Apply changes: copy modified/added files
    for (path, worktree_content) in &worktree_files {
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

    // Apply changes: remove deleted files
    for path in base_tree_files.keys() {
        if !worktree_files.contains_key(path) {
            let dest_path = main_workdir.join(path);
            if dest_path.exists() {
                fs::remove_file(&dest_path)?;
            }
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

/// Detect conflicts between session and main worktree changes
fn detect_conflicts(
    base_tree_files: &std::collections::HashMap<String, Vec<u8>>,
    worktree_files: &std::collections::HashMap<String, Vec<u8>>,
    main_files: &std::collections::HashMap<String, Vec<u8>>,
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
