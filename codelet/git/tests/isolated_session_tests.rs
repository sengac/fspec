//! Feature: spec/features/isolated-session-creation.feature
//!
//! Integration tests for IsolatedSessionInfo with real git repositories.
//! Tests use fixtures (real temp repos) - NO MOCKING.
//!
//! GIT-019: Add isolated parameter to session creation, track worktree info,
//! implement effective_cwd() method.

mod common;

use codelet_git::{GitError, IsolatedSessionInfo, FSPEC_WORKTREES_DIR};
use std::fs;
use std::path::Path;
use std::process::Command;

// =============================================================================
// Test Fixtures
// =============================================================================

/// Get HEAD commit SHA
fn get_head_sha(repo_path: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// =============================================================================
// Scenario: Create isolated session with worktree
// =============================================================================

/// Scenario: Create isolated session with worktree
///
/// @step Given a git repository at "/project"
/// @step And no worktree exists for session "abc123"
/// @step When I create a session with id "abc123" and isolated=true
/// @step Then a worktree should be created at ".fspec/worktrees/abc123/"
/// @step And the session state should include worktree_path
/// @step And the session state should include base_commit
#[test]
fn test_create_isolated_session_with_worktree() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();
    let head_sha = get_head_sha(repo_path);

    // @step And no worktree exists for session "abc123"
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join("abc123");
    assert!(
        !worktree_path.exists(),
        "Precondition: worktree should not exist"
    );

    // @step When I create a session with id "abc123" and isolated=true
    let info = IsolatedSessionInfo::new_isolated(repo_path, "abc123")
        .expect("Failed to create isolated session");

    // @step Then a worktree should be created at ".fspec/worktrees/abc123/"
    assert!(
        worktree_path.exists(),
        "Worktree should be created at .fspec/worktrees/abc123/"
    );

    // @step And the session state should include worktree_path
    assert!(
        info.worktree_path.is_some(),
        "Session should track worktree_path"
    );
    assert_eq!(
        info.worktree_path.as_ref().unwrap(),
        &worktree_path,
        "worktree_path should match expected location"
    );

    // @step And the session state should include base_commit
    assert!(
        info.base_commit.is_some(),
        "Session should track base_commit"
    );
    assert_eq!(
        info.base_commit.as_ref().unwrap(),
        &head_sha,
        "base_commit should match HEAD"
    );

    // Verify isolation flag
    assert!(info.is_isolated(), "Session should be marked as isolated");
}

// =============================================================================
// Scenario: Create non-isolated session without worktree
// =============================================================================

/// Scenario: Create non-isolated session without worktree
///
/// @step Given a git repository at "/project"
/// @step When I create a session with id "def456" and isolated=false
/// @step Then no worktree should be created for session "def456"
/// @step And the session worktree_path should be None
#[test]
fn test_create_non_isolated_session_without_worktree() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step When I create a session with id "def456" and isolated=false
    let info = IsolatedSessionInfo::new_non_isolated(repo_path);

    // @step Then no worktree should be created for session "def456"
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join("def456");
    assert!(
        !worktree_path.exists(),
        "No worktree should be created for non-isolated session"
    );

    // @step And the session worktree_path should be None
    assert!(
        info.worktree_path.is_none(),
        "Non-isolated session should have no worktree_path"
    );
    assert!(
        info.base_commit.is_none(),
        "Non-isolated session should have no base_commit"
    );
    assert!(
        !info.is_isolated(),
        "Session should not be marked as isolated"
    );
}

// =============================================================================
// Scenario: Default session creation is non-isolated
// =============================================================================

/// Scenario: Default session creation is non-isolated
///
/// @step Given a git repository at "/project"
/// @step When I create a session with id "ghi789" without specifying isolation
/// @step Then no worktree should be created for session "ghi789"
/// @step And the session should behave as isolated=false
#[test]
fn test_default_session_creation_is_non_isolated() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step When I create a session with id "ghi789" without specifying isolation
    // Using new_non_isolated as the default factory
    let info = IsolatedSessionInfo::new_non_isolated(repo_path);

    // @step Then no worktree should be created for session "ghi789"
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join("ghi789");
    assert!(
        !worktree_path.exists(),
        "Default session should not create worktree"
    );

    // @step And the session should behave as isolated=false
    assert_eq!(
        info.effective_cwd(),
        repo_path.to_path_buf(),
        "Default session should use project root as effective_cwd"
    );
}

// =============================================================================
// Scenario: effective_cwd returns worktree path for isolated session
// =============================================================================

/// Scenario: effective_cwd returns worktree path for isolated session
///
/// @step Given a git repository at "/project"
/// @step And an isolated session "abc123" with worktree at ".fspec/worktrees/abc123/"
/// @step When I call effective_cwd on the session
/// @step Then the result should be "/project/.fspec/worktrees/abc123"
#[test]
fn test_effective_cwd_returns_worktree_path_for_isolated_session() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And an isolated session "abc123" with worktree at ".fspec/worktrees/abc123/"
    let info = IsolatedSessionInfo::new_isolated(repo_path, "abc123")
        .expect("Failed to create isolated session");

    let expected_path = repo_path.join(FSPEC_WORKTREES_DIR).join("abc123");

    // @step When I call effective_cwd on the session
    let result = info.effective_cwd();

    // @step Then the result should be "/project/.fspec/worktrees/abc123"
    assert_eq!(
        result, expected_path,
        "effective_cwd should return worktree path for isolated session"
    );
}

// =============================================================================
// Scenario: effective_cwd returns project root for non-isolated session
// =============================================================================

/// Scenario: effective_cwd returns project root for non-isolated session
///
/// @step Given a git repository at "/project"
/// @step And a non-isolated session "def456"
/// @step When I call effective_cwd on the session
/// @step Then the result should be "/project"
#[test]
fn test_effective_cwd_returns_project_root_for_non_isolated_session() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And a non-isolated session "def456"
    let info = IsolatedSessionInfo::new_non_isolated(repo_path);

    // @step When I call effective_cwd on the session
    let result = info.effective_cwd();

    // @step Then the result should be "/project"
    assert_eq!(
        result,
        repo_path.to_path_buf(),
        "effective_cwd should return project root for non-isolated session"
    );
}

// =============================================================================
// Scenario: Create isolated session fails if worktree already exists
// =============================================================================

/// Scenario: Create isolated session fails if worktree already exists
///
/// @step Given a git repository at "/project"
/// @step And a worktree already exists for session "abc123"
/// @step When I try to create a session with id "abc123" and isolated=true
/// @step Then the operation should fail with WorktreeExists error
/// @step And the error should reference session "abc123"
#[test]
fn test_create_isolated_session_fails_if_worktree_exists() {
    // @step Given a git repository at "/project"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And a worktree already exists for session "abc123"
    let _first = IsolatedSessionInfo::new_isolated(repo_path, "abc123")
        .expect("Failed to create first isolated session");

    // @step When I try to create a session with id "abc123" and isolated=true
    let result = IsolatedSessionInfo::new_isolated(repo_path, "abc123");

    // @step Then the operation should fail with WorktreeExists error
    assert!(result.is_err(), "Should fail when worktree exists");

    let err = result.unwrap_err();
    match err {
        GitError::WorktreeExists { session_id } => {
            // @step And the error should reference session "abc123"
            assert_eq!(
                session_id, "abc123",
                "Error should reference the session ID"
            );
        }
        other => panic!("Expected WorktreeExists error, got: {:?}", other),
    }
}

// =============================================================================
// Additional Integration Tests
// =============================================================================

/// Test that isolated session at specific commit ref works
#[test]
fn test_create_isolated_session_at_specific_ref() {
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create a second commit
    fs::write(repo_path.join("file2.txt"), "Second file\n").expect("Failed to write file2");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage files");
    Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create second commit");

    // Get first commit SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD~1"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get parent SHA");
    let first_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Create isolated session at first commit
    let info = IsolatedSessionInfo::new_isolated_at_ref(repo_path, "at-first-commit", &first_sha)
        .expect("Failed to create isolated session at ref");

    // Verify base_commit matches the requested ref
    assert_eq!(
        info.base_commit.as_ref().unwrap(),
        &first_sha,
        "base_commit should match requested ref"
    );

    // Verify worktree doesn't have the second file
    let worktree_file2 = info.worktree_path.as_ref().unwrap().join("file2.txt");
    assert!(
        !worktree_file2.exists(),
        "Worktree at old commit should not have file2.txt"
    );
}
