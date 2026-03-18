//! Git diff operations using gitoxide

use crate::error::{GitError, Result};
use crate::open_repo;
use crate::utils::is_binary_content;
use similar::{ChangeTag, TextDiff};
use std::path::Path;

/// Get unified diff for a file comparing working directory to HEAD
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `filepath` - Path to the file (relative to repository root)
///
/// # Returns
/// Unified diff string, or None if no changes
pub fn get_file_diff(dir: impl AsRef<Path>, filepath: &str) -> Result<Option<String>> {
    let repo = open_repo(dir.as_ref())?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| GitError::Other("Not a worktree".to_string()))?;

    let full_path = workdir.join(filepath);

    // Read working directory content
    if !full_path.exists() {
        return Err(GitError::FileNotFound(filepath.to_string()));
    }

    let working_content = std::fs::read(&full_path)?;

    // Check for binary content
    if is_binary_content(&working_content) {
        return Ok(Some("[Binary file - no diff available]".to_string()));
    }

    let working_str = String::from_utf8_lossy(&working_content);

    // Get HEAD content
    let head_content = get_head_file_content(&repo, filepath).unwrap_or_default();

    // Check if HEAD content is binary
    if is_binary_content(head_content.as_bytes()) {
        return Ok(Some("[Binary file - no diff available]".to_string()));
    }

    // If contents are identical, no diff
    if head_content == working_str {
        return Ok(None);
    }

    // Generate unified diff
    let diff = generate_unified_diff(filepath, &head_content, &working_str);
    Ok(Some(diff))
}

/// Check if file content is binary
///
/// Uses null byte detection to identify binary files
pub fn is_binary_file(dir: impl AsRef<Path>, filepath: &str) -> Result<bool> {
    let workdir = dir.as_ref();
    let full_path = workdir.join(filepath);

    if !full_path.exists() {
        return Err(GitError::FileNotFound(filepath.to_string()));
    }

    let content = std::fs::read(&full_path)?;
    Ok(is_binary_content(&content))
}

/// Get file content from HEAD commit
fn get_head_file_content(repo: &gix::Repository, filepath: &str) -> Result<String> {
    let head = repo
        .head_commit()
        .map_err(|e| GitError::Head(e.to_string()))?;

    let tree = head.tree().map_err(|e| GitError::Head(e.to_string()))?;

    let entry = tree
        .find_entry(filepath)
        .ok_or_else(|| GitError::FileNotFound(filepath.to_string()))?;

    let object = entry.object().map_err(|e| GitError::ReadBlob {
        path: filepath.to_string(),
        source: e.into(),
    })?;

    let blob = object.into_blob();
    let content = String::from_utf8_lossy(blob.data.as_ref()).to_string();

    Ok(content)
}

/// Get file content from an arbitrary commit ref
fn get_ref_file_content(
    repo: &gix::Repository,
    commit_ref: &str,
    filepath: &str,
) -> Result<String> {
    let commit_id = repo
        .rev_parse_single(commit_ref.as_bytes())
        .map_err(|e| GitError::Other(format!("Failed to resolve ref '{}': {}", commit_ref, e)))?;

    let commit = repo
        .find_object(commit_id)
        .map_err(|e| GitError::Other(format!("Failed to find object '{}': {}", commit_ref, e)))?
        .try_into_commit()
        .map_err(|e| GitError::Other(format!("Not a commit '{}': {}", commit_ref, e)))?;

    let tree = commit
        .tree()
        .map_err(|e| GitError::Other(format!("Failed to get tree for '{}': {}", commit_ref, e)))?;

    let entry = tree
        .find_entry(filepath)
        .ok_or_else(|| GitError::FileNotFound(filepath.to_string()))?;

    let object = entry.object().map_err(|e| GitError::ReadBlob {
        path: filepath.to_string(),
        source: e.into(),
    })?;

    let blob = object.into_blob();
    let content = String::from_utf8_lossy(blob.data.as_ref()).to_string();

    Ok(content)
}

/// Get unified diff for a single file between HEAD and a checkpoint commit
///
/// Shows what would change if the checkpoint were restored:
/// HEAD content is shown as "old" (lines removed on restore),
/// checkpoint content is shown as "new" (lines added on restore).
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `filepath` - Path to the file (relative to repository root)
/// * `checkpoint_ref` - Full ref or SHA of the checkpoint commit
///
/// # Returns
/// Unified diff string, or None if no changes
pub fn get_checkpoint_file_diff(
    dir: impl AsRef<Path>,
    filepath: &str,
    checkpoint_ref: &str,
) -> Result<Option<String>> {
    let repo = open_repo(dir.as_ref())?;

    // Read file content from HEAD
    let head_content = get_head_file_content(&repo, filepath);

    // Read file content from checkpoint
    let checkpoint_content = get_ref_file_content(&repo, checkpoint_ref, filepath);

    match (&head_content, &checkpoint_content) {
        // File not in checkpoint — restoring would delete it
        (Ok(_), Err(_)) => {
            Ok(Some("[File will be deleted on restore]".to_string()))
        }
        // File not in HEAD but exists in checkpoint — restoring would create it
        (Err(_), Ok(cp_str)) => {
            if is_binary_content(cp_str.as_bytes()) {
                return Ok(Some("[Binary file - no diff available]".to_string()));
            }
            let diff = generate_unified_diff(filepath, "", cp_str);
            Ok(Some(diff))
        }
        // File exists in neither — shouldn't happen, but handle gracefully
        (Err(_), Err(_)) => {
            Ok(Some("[File will be deleted on restore]".to_string()))
        }
        // File exists in both — compare contents
        (Ok(head_str), Ok(cp_str)) => {
            // Check for binary content
            if is_binary_content(head_str.as_bytes()) || is_binary_content(cp_str.as_bytes()) {
                return Ok(Some("[Binary file - no diff available]".to_string()));
            }

            // If contents are identical, no diff
            if head_str == cp_str {
                return Ok(Some("[No changes - file is identical]".to_string()));
            }

            // Generate unified diff: HEAD as old, checkpoint as new (restore preview)
            let diff = generate_unified_diff(filepath, head_str, cp_str);
            Ok(Some(diff))
        }
    }
}

/// Generate unified diff format from two strings
fn generate_unified_diff(_filepath: &str, old_content: &str, new_content: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);

    let mut added = 0;
    let mut removed = 0;
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                removed += 1;
                lines.push(format!("-{}", change.value().trim_end_matches('\n')));
            }
            ChangeTag::Insert => {
                added += 1;
                lines.push(format!("+{}", change.value().trim_end_matches('\n')));
            }
            ChangeTag::Equal => {
                lines.push(format!(" {}", change.value().trim_end_matches('\n')));
            }
        }
    }

    // Truncate if more than 20,000 lines
    const MAX_LINES: usize = 20000;
    let total_lines = lines.len();
    let truncated = total_lines > MAX_LINES;

    if truncated {
        lines.truncate(MAX_LINES);
    }

    let mut result = Vec::new();
    result.push(format!(
        "--- Lines that will be REMOVED on restore: {} lines",
        removed
    ));
    result.push(format!(
        "+++ Lines that will be ADDED on restore: {} lines",
        added
    ));
    result.extend(lines);

    if truncated {
        result.push(String::new());
        result.push(format!(
            "[File truncated - showing first {} of {} lines]",
            MAX_LINES, total_lines
        ));
    }

    result.join("\n")
}
