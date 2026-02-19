//! Tests for session result collection and patch application
//!
//! Feature: Session result collection and patch application
//! spec/features/session-result-collection-and-patch-application.feature

mod common;

use codelet_git::{
    abort_session, apply_session_changes, create_worktree, get_session_diff, GitError,
    FSPEC_WORKTREES_DIR,
};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to get HEAD commit SHA
fn get_head_sha(repo_path: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to get HEAD");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// =============================================================================
// Scenario: Get session diff with changes
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-001/"
/// @step And the session worktree has modified file "src/main.rs"
/// @step And the session worktree has added file "src/new-feature.rs"
/// @step And the session worktree has deleted file "src/deprecated.rs"
/// @step When I get the session diff for "session-001"
/// @step Then I should receive a SessionResult
/// @step And the SessionResult should contain a unified diff
/// @step And the diff should show "src/main.rs" as modified
/// @step And the diff should show "src/new-feature.rs" as added
/// @step And the diff should show "src/deprecated.rs" as deleted
/// @step And the SessionResult should contain the base_commit
/// @step And the SessionResult should contain files_changed count
/// @step And the SessionResult should contain files_added count
/// @step And the SessionResult should contain files_deleted count
/// @step And the session worktree should still exist
#[test]
fn test_get_session_diff_with_changes() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Add a file that will be deleted in session
    let src_dir = repo_path.join("src");
    fs::write(src_dir.join("deprecated.rs"), "// deprecated\n")
        .expect("Failed to write deprecated.rs");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage");
    Command::new("git")
        .args(["commit", "-m", "Add deprecated file"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    let _base_commit = get_head_sha(repo_path);

    // And a session worktree exists at ".fspec/worktrees/session-001/"
    let session_id = "session-001";
    let create_result = create_worktree(repo_path, session_id);
    assert!(
        create_result.is_ok(),
        "Failed to create worktree: {:?}",
        create_result.err()
    );

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has modified file "src/main.rs"
    fs::write(
        worktree_path.join("src/main.rs"),
        "fn main() { println!(\"modified\"); }\n",
    )
    .expect("Failed to modify main.rs");

    // And the session worktree has added file "src/new-feature.rs"
    fs::write(
        worktree_path.join("src/new-feature.rs"),
        "pub fn new_feature() {}\n",
    )
    .expect("Failed to add new-feature.rs");

    // And the session worktree has deleted file "src/deprecated.rs"
    fs::remove_file(worktree_path.join("src/deprecated.rs"))
        .expect("Failed to delete deprecated.rs");

    // When I get the session diff for "session-001"
    let result = get_session_diff(repo_path, session_id);
    assert!(
        result.is_ok(),
        "get_session_diff failed: {:?}",
        result.err()
    );
    let session_result = result.unwrap();

    // Then I should receive a SessionResult
    // And the SessionResult should contain a unified diff
    assert!(!session_result.diff.is_empty());

    // And the diff should show files as modified/added/deleted
    assert!(session_result
        .files_changed
        .contains(&"src/main.rs".to_string()));
    assert!(session_result
        .files_added
        .contains(&"src/new-feature.rs".to_string()));
    assert!(session_result
        .files_deleted
        .contains(&"src/deprecated.rs".to_string()));

    // And the SessionResult should contain the base_commit
    assert!(!session_result.base_commit.is_empty());

    // And the session worktree should still exist
    assert!(
        worktree_path.exists(),
        "Worktree should still exist after get_session_diff"
    );
}

// =============================================================================
// Scenario: Apply session changes copies modified files
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-002/"
/// @step And the session worktree has modified file "src/lib.rs" with content "updated content"
/// @step And the main repository "src/lib.rs" has not changed since session creation
/// @step When I apply session changes from "session-002" to main worktree
/// @step Then the main repository "src/lib.rs" should contain "updated content"
/// @step And the session worktree should be removed
/// @step And the git worktree metadata should be cleaned up
#[test]
fn test_apply_session_changes_copies_modified_files() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create lib.rs in main repo
    let src_dir = repo_path.join("src");
    fs::write(src_dir.join("lib.rs"), "// original content\n").expect("Failed to write lib.rs");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage");
    Command::new("git")
        .args(["commit", "-m", "Add lib.rs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    // And a session worktree exists at ".fspec/worktrees/session-002/"
    let session_id = "session-002";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has modified file "src/lib.rs"
    let updated_content = "// updated content\npub fn updated() {}\n";
    fs::write(worktree_path.join("src/lib.rs"), updated_content)
        .expect("Failed to modify lib.rs in worktree");

    // When I apply session changes from "session-002" to main worktree
    let result = apply_session_changes(repo_path, session_id);
    assert!(
        result.is_ok(),
        "apply_session_changes failed: {:?}",
        result.err()
    );

    // Then the main repository "src/lib.rs" should contain "updated content"
    let main_content = fs::read_to_string(src_dir.join("lib.rs")).unwrap();
    assert!(main_content.contains("updated content"));

    // And the session worktree should be removed
    assert!(!worktree_path.exists());
}

// =============================================================================
// Scenario: Apply session changes copies added files
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-002a/"
/// @step And the session worktree has added file "src/new-module.rs" with content "new code"
/// @step When I apply session changes from "session-002a" to main worktree
/// @step Then the main repository should have file "src/new-module.rs"
/// @step And the main repository "src/new-module.rs" should contain "new code"
/// @step And the session worktree should be removed
#[test]
fn test_apply_session_changes_copies_added_files() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists
    let session_id = "session-002a";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has added file "src/new-module.rs"
    let new_content = "// new code\npub fn new_module() {}\n";
    fs::write(worktree_path.join("src/new-module.rs"), new_content)
        .expect("Failed to add new-module.rs");

    // When I apply session changes
    let result = apply_session_changes(repo_path, session_id);
    assert!(
        result.is_ok(),
        "apply_session_changes failed: {:?}",
        result.err()
    );

    // Then main repo should have the new file
    let main_path = repo_path.join("src/new-module.rs");
    assert!(main_path.exists());
    let content = fs::read_to_string(&main_path).unwrap();
    assert!(content.contains("new code"));
}

// =============================================================================
// Scenario: Apply session changes removes deleted files
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And the main repository has file "src/old-module.rs"
/// @step And a session worktree exists at ".fspec/worktrees/session-002b/"
/// @step And the session worktree has deleted file "src/old-module.rs"
/// @step When I apply session changes from "session-002b" to main worktree
/// @step Then the main repository should NOT have file "src/old-module.rs"
/// @step And the session worktree should be removed
#[test]
fn test_apply_session_changes_removes_deleted_files() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And the main repository has file "src/old-module.rs"
    let src_dir = repo_path.join("src");
    fs::write(src_dir.join("old-module.rs"), "// old module\n")
        .expect("Failed to write old-module.rs");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage");
    Command::new("git")
        .args(["commit", "-m", "Add old-module"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    // And a session worktree exists
    let session_id = "session-002b";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has deleted file "src/old-module.rs"
    fs::remove_file(worktree_path.join("src/old-module.rs"))
        .expect("Failed to delete old-module.rs");

    // When I apply session changes
    let result = apply_session_changes(repo_path, session_id);
    assert!(
        result.is_ok(),
        "apply_session_changes failed: {:?}",
        result.err()
    );

    // Then main repo should NOT have the file
    assert!(!src_dir.join("old-module.rs").exists());
}

// =============================================================================
// Scenario: Apply session changes when main has diverged
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-003/" based on commit "abc123"
/// @step And the session worktree has modified file "src/conflict.rs"
/// @step And the main repository has also modified "src/conflict.rs" since commit "abc123"
/// @step When I attempt to apply session changes from "session-003"
/// @step Then I should receive a conflict error
/// @step And the error should list "src/conflict.rs" as conflicting
/// @step And the session worktree should NOT be removed
/// @step And the main repository should be unchanged
#[test]
fn test_apply_session_changes_conflict_detection() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Create conflict.rs
    let src_dir = repo_path.join("src");
    fs::write(src_dir.join("conflict.rs"), "// original\n").expect("Failed to write conflict.rs");
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to stage");
    Command::new("git")
        .args(["commit", "-m", "Add conflict.rs"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    // And a session worktree exists
    let session_id = "session-003";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has modified the file
    fs::write(worktree_path.join("src/conflict.rs"), "// session change\n")
        .expect("Failed to modify in worktree");

    // And the main repository has also modified the file
    fs::write(src_dir.join("conflict.rs"), "// main change\n").expect("Failed to modify in main");

    // When I attempt to apply session changes
    let result = apply_session_changes(repo_path, session_id);
    assert!(result.is_err());

    // Then I should receive a conflict error
    match result.unwrap_err() {
        GitError::ConflictError { files } => {
            assert!(files.contains(&"src/conflict.rs".to_string()));
        }
        e => panic!("Expected ConflictError, got: {:?}", e),
    }

    // And the session worktree should NOT be removed
    assert!(
        worktree_path.exists(),
        "Worktree should exist after conflict"
    );
}

// =============================================================================
// Scenario: Abort session discards changes
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-004/"
/// @step And the session worktree has modified file "src/work-in-progress.rs"
/// @step When I abort the session "session-004"
/// @step Then the ".fspec/worktrees/session-004/" directory should not exist
/// @step And the git worktree metadata should be cleaned up
/// @step And the main repository should be unchanged
#[test]
fn test_abort_session_discards_changes() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists
    let session_id = "session-004";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    let git_meta_path = repo_path.join(".git/worktrees").join(session_id);

    // And the session worktree has modified file
    fs::write(worktree_path.join("src/main.rs"), "// work in progress\n")
        .expect("Failed to modify file");

    // Save original main repo state
    let original_main =
        fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read main.rs");

    // When I abort the session
    // When I abort the session
    let result = abort_session(repo_path, session_id);
    assert!(result.is_ok());

    // Then the worktree directory should not exist
    assert!(!worktree_path.exists());

    // And the git worktree metadata should be cleaned up
    assert!(!git_meta_path.exists());

    // And the main repository should be unchanged
    let current_main =
        fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read main.rs");
    assert_eq!(original_main, current_main);
}

// =============================================================================
// Scenario: Get session diff with no changes
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-005/"
/// @step And the session worktree has no changes from base_commit
/// @step When I get the session diff for "session-005"
/// @step Then I should receive a SessionResult
/// @step And the SessionResult diff should be empty
/// @step And the SessionResult files_changed should be 0
/// @step And the SessionResult files_added should be 0
/// @step And the SessionResult files_deleted should be 0
#[test]
fn test_get_session_diff_no_changes() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists (with no modifications)
    let session_id = "session-005";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    // When I get the session diff
    let result = get_session_diff(repo_path, session_id);
    assert!(
        result.is_ok(),
        "get_session_diff failed: {:?}",
        result.err()
    );
    let session_result = result.unwrap();

    // Then the diff should be empty
    assert!(session_result.diff.is_empty() || session_result.diff.trim().is_empty());
    assert_eq!(session_result.files_changed.len(), 0);
    assert_eq!(session_result.files_added.len(), 0);
    assert_eq!(session_result.files_deleted.len(), 0);
}

// =============================================================================
// Scenario: Session diff handles binary files
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-006/"
/// @step And the session worktree has added binary file "assets/image.png"
/// @step When I get the session diff for "session-006"
/// @step Then the diff should indicate "assets/image.png" is a binary file
/// @step And the diff should NOT contain binary content
#[test]
fn test_session_diff_handles_binary_files() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists
    let session_id = "session-006";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has added binary file
    let assets_dir = worktree_path.join("assets");
    fs::create_dir_all(&assets_dir).expect("Failed to create assets dir");

    // Create a simple binary file (PNG header + garbage)
    let binary_content: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG header
        0x00, 0x00, 0x00, 0x00, // Some binary data
    ];
    fs::write(assets_dir.join("image.png"), &binary_content).expect("Failed to write binary file");

    // When I get the session diff
    let result = get_session_diff(repo_path, session_id);
    assert!(
        result.is_ok(),
        "get_session_diff failed: {:?}",
        result.err()
    );
    let session_result = result.unwrap();

    // Then the diff should indicate binary file
    assert!(session_result.diff.contains("Binary file") || session_result.diff.contains("binary"));
    assert!(session_result
        .files_added
        .contains(&"assets/image.png".to_string()));
}

// =============================================================================
// Scenario: Get session diff without applying
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And a session worktree exists at ".fspec/worktrees/session-007/"
/// @step And the session worktree has multiple changes
/// @step When I get the session diff for "session-007"
/// @step Then I should receive a unified diff
/// @step And the session worktree should still exist
/// @step And the main repository should be unchanged
#[test]
fn test_get_session_diff_without_applying() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists
    let session_id = "session-007";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has multiple changes
    fs::write(worktree_path.join("src/main.rs"), "// changed\n").expect("Failed to modify");
    fs::write(worktree_path.join("new-file.txt"), "new content\n").expect("Failed to add");

    // Save original main repo state
    let original_main = fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read");

    // When I get the session diff
    let result = get_session_diff(repo_path, session_id);
    assert!(
        result.is_ok(),
        "get_session_diff failed: {:?}",
        result.err()
    );
    let _session_result = result.unwrap();

    // Then the session worktree should still exist
    assert!(worktree_path.exists());

    // And the main repository should be unchanged
    let current_main = fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read");
    assert_eq!(original_main, current_main);
    assert!(!repo_path.join("new-file.txt").exists());
}

// =============================================================================
// Scenario: Fail gracefully when session does not exist
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And no session worktree exists for "nonexistent-session"
/// @step When I attempt to get session diff for "nonexistent-session"
/// @step Then I should receive an error indicating worktree not found
#[test]
fn test_fail_gracefully_session_not_found_diff() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And no session worktree exists for "nonexistent-session"
    let session_id = "nonexistent-session";
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
    assert!(!worktree_path.exists());

    // When I attempt to get session diff
    let result = get_session_diff(repo_path, session_id);
    assert!(result.is_err());
    match result.unwrap_err() {
        GitError::WorktreeNotFound { session_id: sid } => {
            assert_eq!(sid, session_id);
        }
        e => panic!("Expected WorktreeNotFound, got: {:?}", e),
    }
}

// =============================================================================
// Scenario: Fail gracefully when applying non-existent session
// =============================================================================

/// @step Given I have a git repository with commits
/// @step And no session worktree exists for "nonexistent-session"
/// @step When I attempt to apply session changes from "nonexistent-session"
/// @step Then I should receive an error indicating worktree not found
#[test]
fn test_fail_gracefully_session_not_found_apply() {
    // Given I have a git repository with commits
    let tmp_dir = common::setup_test_repo();
    let _repo_path = tmp_dir.path();

    // And no session worktree exists
    let session_id = "nonexistent-session";

    // When I attempt to apply session changes
    let result = apply_session_changes(_repo_path, session_id);
    assert!(result.is_err());
    match result.unwrap_err() {
        GitError::WorktreeNotFound { session_id: sid } => {
            assert_eq!(sid, session_id);
        }
        e => panic!("Expected WorktreeNotFound, got: {:?}", e),
    }
}

// =============================================================================
// NAPI BINDING TESTS
// Note: These test the public API that will be exposed via NAPI. The actual
// NAPI bindings will call these functions, so testing the functions here
// validates the API contract.
// =============================================================================

// =============================================================================
// Scenario: NAPI binding exposes getSessionDiff
// =============================================================================

/// @step Given the codelet-napi module is loaded
/// @step And a session worktree exists at ".fspec/worktrees/napi-test-1/"
/// @step When I call getSessionDiff via NAPI with session ID "napi-test-1"
/// @step Then I should receive a SessionResult object with diff property
#[test]
fn test_napi_get_session_diff_binding() {
    // Given the codelet-napi module is loaded (tested via Rust API here)
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists at ".fspec/worktrees/napi-test-1/"
    let session_id = "napi-test-1";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Modify a file in the worktree
    fs::write(
        worktree_path.join("src/main.rs"),
        "// modified for NAPI test\n",
    )
    .expect("Failed to modify file");

    // When I call getSessionDiff (tested via the Rust function that NAPI wraps)
    let result = get_session_diff(repo_path, session_id);
    assert!(
        result.is_ok(),
        "get_session_diff failed: {:?}",
        result.err()
    );
    let session_result = result.unwrap();

    // Then I should receive a SessionResult object with diff property
    assert!(!session_result.diff.is_empty());
    assert_eq!(session_result.session_id, session_id);
}

// =============================================================================
// Scenario: NAPI binding exposes applySessionChanges
// =============================================================================

/// @step Given the codelet-napi module is loaded
/// @step And a session worktree exists at ".fspec/worktrees/napi-test-2/"
/// @step And the session worktree has modified file "src/test.rs"
/// @step When I call applySessionChanges via NAPI with session ID "napi-test-2"
/// @step Then the changes should be applied to the main worktree
/// @step And the session worktree should be removed
#[test]
fn test_napi_apply_session_changes_binding() {
    // Given the codelet-napi module is loaded
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // And a session worktree exists
    let session_id = "napi-test-2";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // And the session worktree has modified file "src/test.rs"
    fs::write(worktree_path.join("src/main.rs"), "// NAPI test changes\n")
        .expect("Failed to modify file");

    // When I call applySessionChanges
    let result = apply_session_changes(repo_path, session_id);
    assert!(
        result.is_ok(),
        "apply_session_changes failed: {:?}",
        result.err()
    );

    // Then the changes should be applied to the main worktree
    let main_content = fs::read_to_string(repo_path.join("src/main.rs")).unwrap();
    assert!(main_content.contains("NAPI test changes"));

    // And the session worktree should be removed
    assert!(!worktree_path.exists());
}

// =============================================================================
// Scenario: NAPI binding exposes abortSession
// =============================================================================

/// @step Given the codelet-napi module is loaded
/// @step And a session worktree exists at ".fspec/worktrees/napi-test-3/"
/// @step When I call abortSession via NAPI with session ID "napi-test-3"
/// @step Then the session worktree should be removed
/// @step And the main repository should be unchanged
#[test]
fn test_napi_abort_session_binding() {
    // Given the codelet-napi module is loaded
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // Save original main repo state
    let original_main = fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read");

    // And a session worktree exists
    let session_id = "napi-test-3";
    let create_result = create_worktree(repo_path, session_id);
    assert!(create_result.is_ok());

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);

    // Make some changes in the worktree
    fs::write(worktree_path.join("src/main.rs"), "// changes to discard\n")
        .expect("Failed to modify");

    // When I call abortSession
    let result = abort_session(repo_path, session_id);
    assert!(result.is_ok());

    // Then the session worktree should be removed
    assert!(!worktree_path.exists());

    // And the main repository should be unchanged
    let current_main = fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read");
    assert_eq!(original_main, current_main);
}
