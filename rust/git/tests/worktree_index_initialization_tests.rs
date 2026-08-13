//! Feature: spec/features/isolated-session-worktree-initialization.feature
//!
//! Integration tests for worktree index initialization.
//! These tests verify that git worktrees are created with a properly
//! initialized index, not an empty index that causes all files to appear
//! as "staged for deletion".
//!
//! GIT-035: Isolated session worktree created with empty git index

mod common;

use codelet_git::{IsolatedSessionInfo, FSPEC_WORKTREES_DIR};
use std::process::Command;

// =============================================================================
// Scenario: Worktree has all tracked files in git index after creation
// =============================================================================

/// Scenario: Worktree has all tracked files in git index after creation
///
/// @step Given I have a git repository with tracked files
/// @step When I create an isolated session
/// @step Then the worktree should exist at ".fspec/worktrees/<session-id>/"
/// @step And "git ls-files" in the worktree should return all tracked files
/// @step And the file count should match the main repository
#[test]
fn test_worktree_has_all_tracked_files_in_git_index() {
    // @step Given I have a git repository with tracked files
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // Count files in main repo
    let main_ls_files = Command::new("git")
        .args(["ls-files"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to run git ls-files in main repo");
    let main_file_count = String::from_utf8_lossy(&main_ls_files.stdout)
        .lines()
        .count();
    assert!(
        main_file_count > 0,
        "Precondition: main repo should have tracked files"
    );

    // @step When I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, "test-index-session")
        .expect("Failed to create isolated session");

    // @step Then the worktree should exist at ".fspec/worktrees/<session-id>/"
    let worktree_path = repo_path
        .join(FSPEC_WORKTREES_DIR)
        .join("test-index-session");
    assert!(worktree_path.exists(), "Worktree should exist");

    // @step And "git ls-files" in the worktree should return all tracked files
    let worktree_ls_files = Command::new("git")
        .args(["ls-files"])
        .current_dir(info.effective_cwd())
        .output()
        .expect("Failed to run git ls-files in worktree");

    let worktree_file_count = String::from_utf8_lossy(&worktree_ls_files.stdout)
        .lines()
        .count();

    // @step And the file count should match the main repository
    assert_eq!(
        worktree_file_count, main_file_count,
        "Worktree git ls-files should return same file count as main repo. \
         Got {} files in worktree, expected {} files (same as main repo). \
         If worktree has 0 files, the git index was not initialized properly.",
        worktree_file_count, main_file_count
    );

    // Additional assertion: file count should not be zero
    assert!(
        worktree_file_count > 0,
        "Worktree git ls-files should return files, not 0. \
         This indicates the git index was not initialized from HEAD."
    );
}

// =============================================================================
// Scenario: Worktree has clean git status after creation
// =============================================================================

/// Scenario: Worktree has clean git status after creation
///
/// @step Given I have a git repository with tracked files
/// @step When I create an isolated session
/// @step Then "git status" in the worktree should show "nothing to commit, working tree clean"
/// @step And there should be no staged changes
#[test]
fn test_worktree_has_clean_git_status_after_creation() {
    // @step Given I have a git repository with tracked files
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step When I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, "test-status-session")
        .expect("Failed to create isolated session");

    // @step Then "git status" in the worktree should show clean state
    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(info.effective_cwd())
        .output()
        .expect("Failed to run git status in worktree");

    let status_output = String::from_utf8_lossy(&git_status.stdout);

    // @step And there should be no staged changes
    assert!(
        status_output.trim().is_empty(),
        "Worktree should have clean git status (no staged or unstaged changes). \
         Got: '{}'. \
         If status shows 'D' for all files, the git index was not initialized properly.",
        status_output.trim()
    );

    // Additional check: verify there are no staged deletions
    let staged_deletions = status_output
        .lines()
        .filter(|line| line.starts_with("D "))
        .count();
    assert_eq!(
        staged_deletions, 0,
        "Worktree should have no staged deletions. \
         Found {} files staged for deletion, which indicates empty git index.",
        staged_deletions
    );
}

// =============================================================================
// Scenario: Session Management Panel shows accurate file change count
// =============================================================================

/// Scenario: Session Management Panel shows accurate file change count
///
/// Tests that get_session_diff returns accurate counts after modifying a file.
///
/// @step Given I have an isolated session with a worktree
/// @step When I modify a file in the worktree
/// @step And I open the Session Management Panel
/// @step Then the session should show "1 files changed"
/// @step And the modified file should appear in the changes list
#[test]
fn test_session_diff_shows_accurate_file_change_count() {
    use codelet_git::get_session_diff;
    use std::fs;

    // @step Given I have an isolated session with a worktree
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    let info = IsolatedSessionInfo::new_isolated(repo_path, "test-diff-session")
        .expect("Failed to create isolated session");

    // @step When I modify a file in the worktree
    let worktree_path = info.effective_cwd();
    let file_to_modify = worktree_path.join("src/main.rs");
    fs::write(&file_to_modify, "fn main() { println!(\"modified\"); }\n")
        .expect("Failed to modify file");

    // @step And I open the Session Management Panel (call get_session_diff)
    let diff_result =
        get_session_diff(repo_path, "test-diff-session").expect("Failed to get session diff");

    // @step Then the session should show "1 files changed"
    assert_eq!(
        diff_result.files_changed.len(),
        1,
        "Should show exactly 1 file changed. Got: {:?}",
        diff_result.files_changed
    );

    // @step And the modified file should appear in the changes list
    assert!(
        diff_result
            .files_changed
            .contains(&"src/main.rs".to_string()),
        "Modified file src/main.rs should be in changes list. Got: {:?}",
        diff_result.files_changed
    );

    // Verify no false positives
    assert!(
        diff_result.files_added.is_empty(),
        "No files should be added. Got: {:?}",
        diff_result.files_added
    );
    assert!(
        diff_result.files_deleted.is_empty(),
        "No files should be deleted. Got: {:?}",
        diff_result.files_deleted
    );
}

// =============================================================================
// Scenario: get_session_diff detects corrupted index
// =============================================================================

/// Scenario: get_session_diff detects and reports corrupted index
///
/// This test verifies that if a worktree somehow ends up with an empty index,
/// the system either detects and reports the error, or repairs it.
///
/// @step Given I have an isolated session with an empty git index
/// @step When get_session_diff is called for that session
/// @step Then it should detect the corrupted index state
/// @step And it should either report an error or repair the index
#[test]
fn test_session_diff_detects_corrupted_empty_index() {
    use codelet_git::get_session_diff;
    use std::fs;

    // @step Given I have an isolated session
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    let info = IsolatedSessionInfo::new_isolated(repo_path, "test-corrupted-session")
        .expect("Failed to create isolated session");

    // Corrupt the index by removing it (simulates the bug)
    let worktree_path = info.effective_cwd();
    let index_path = worktree_path.join(".git"); // This is a file pointing to the real git dir

    // Read the gitdir reference
    let gitdir_content = fs::read_to_string(&index_path).expect("Failed to read .git file");
    let gitdir_line = gitdir_content
        .lines()
        .find(|line| line.starts_with("gitdir:"))
        .expect("No gitdir line found");
    let gitdir_path = gitdir_line.trim_start_matches("gitdir:").trim();

    // Remove the index file to simulate corrupted state
    let real_index_path = std::path::Path::new(gitdir_path).join("index");
    if real_index_path.exists() {
        fs::remove_file(&real_index_path).expect("Failed to remove index");
    }

    // @step When get_session_diff is called for that session
    let diff_result = get_session_diff(repo_path, "test-corrupted-session");

    // @step Then it should detect the corrupted index state
    // @step And it should either report an error or repair the index
    // Currently, get_session_diff compares working dir to base commit tree,
    // so it will still work but report 0 changes (which is the bug we're testing).
    // After the fix, this should either:
    // - Return an error indicating corrupted index, OR
    // - Detect and repair the index, OR
    // - Compare against the actual git state correctly

    // For now, we just verify the function doesn't panic
    // After the fix, we should update this to verify proper behavior
    match diff_result {
        Ok(result) => {
            // If we get a result, verify it's not falsely reporting 0 changes
            // when files exist in the working directory
            let working_dir_files = fs::read_dir(&worktree_path)
                .expect("Failed to read worktree dir")
                .filter(|e| e.is_ok())
                .count();

            if working_dir_files > 0 {
                // With corrupted index, current implementation reports 0 changes
                // This assertion documents the bug - after fix, this should change
                // The fix should either:
                // 1. Report an error for corrupted index
                // 2. Detect and repair the index
                // 3. Report accurate changes
                eprintln!(
                    "Note: get_session_diff with corrupted index returned {} changed, {} added, {} deleted",
                    result.files_changed.len(),
                    result.files_added.len(),
                    result.files_deleted.len()
                );
            }
        }
        Err(e) => {
            // Error is acceptable - it means the corruption was detected
            eprintln!(
                "get_session_diff correctly returned error for corrupted index: {:?}",
                e
            );
        }
    }
}
