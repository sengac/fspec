//! Feature: spec/features/session-discard-operations.feature
//!
//! Integration tests for session discard operations.
//! Tests use fixtures (real temp repos) - NO MOCKING.
//!
//! GIT-025: Discard session without applying changes.

mod common;

use codelet_git::{
    create_session_manifest, discard_session, get_manifest_path, DerivedSessionStatus,
    IsolatedSessionInfo,
};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// Test Fixtures
// =============================================================================

/// Atomic counter to generate unique session IDs for parallel tests
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique session ID for testing
fn unique_session_id(prefix: &str) -> String {
    let count = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{prefix}_{count}_{}", std::process::id())
}

// =============================================================================
// Scenario: Discard session without applying changes
// =============================================================================

/// Scenario: Discard session without applying changes
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with a modified file "src/main.rs"
/// @step When I call discard_session with the session ID
/// @step Then the session worktree should be removed
/// @step And the main worktree should NOT contain the modified content
/// @step And the DiscardResult should contain files_discarded greater than 0
#[test]
fn test_discard_session_without_applying_changes() {
    let session_id = unique_session_id("discard_changes");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // Save original main.rs content
    let original_content =
        fs::read_to_string(repo_path.join("src/main.rs")).expect("Failed to read original main.rs");

    // @step And a session worktree with a modified file "src/main.rs"
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");

    // Modify src/main.rs in session worktree
    fs::write(
        worktree_path.join("src/main.rs"),
        "fn main() { println!(\"DISCARDED CHANGES\"); }\n",
    )
    .expect("Failed to modify main.rs in worktree");

    // @step When I call discard_session with the session ID
    let result = discard_session(repo_path, &session_id).expect("Failed to discard session");

    // @step Then the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after discard"
    );

    // @step And the main worktree should NOT contain the modified content
    let main_content = fs::read_to_string(repo_path.join("src/main.rs"))
        .expect("Failed to read main.rs from main worktree");
    assert_eq!(
        main_content, original_content,
        "Main worktree should still have original content"
    );
    assert!(
        !main_content.contains("DISCARDED CHANGES"),
        "Main worktree should NOT contain discarded changes"
    );

    // @step And the DiscardResult should contain files_discarded greater than 0
    assert!(
        result.files_discarded > 0,
        "files_discarded should be > 0, got: {}",
        result.files_discarded
    );
}

// =============================================================================
// Scenario: Discard clean session without confirmation
// =============================================================================

/// Scenario: Discard clean session without confirmation
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with no changes
/// @step When I call discard_session with the session ID
/// @step Then the session worktree should be removed
/// @step And the DiscardResult should contain files_discarded equal to 0
#[test]
fn test_discard_clean_session() {
    let session_id = unique_session_id("discard_clean");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree with no changes
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");

    // No changes made to worktree - it's clean

    // @step When I call discard_session with the session ID
    let result = discard_session(repo_path, &session_id).expect("Failed to discard clean session");

    // @step Then the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after discard"
    );

    // @step And the DiscardResult should contain files_discarded equal to 0
    assert_eq!(
        result.files_discarded, 0,
        "files_discarded should be 0 for clean session, got: {}",
        result.files_discarded
    );
}

// =============================================================================
// Scenario: Discard session fails for non-existent session
// =============================================================================

/// Scenario: Discard session fails for non-existent session
///
/// @step Given a git repository with an initial commit
/// @step And a session ID that does not exist
/// @step When I call discard_session with the non-existent session ID
/// @step Then I should receive a WorktreeNotFound error
#[test]
fn test_discard_session_fails_for_nonexistent() {
    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session ID that does not exist
    let non_existent_session = "non_existent_session_xyz";

    // @step When I call discard_session with the non-existent session ID
    let result = discard_session(repo_path, non_existent_session);

    // @step Then I should receive a WorktreeNotFound error
    assert!(
        result.is_err(),
        "discard_session should fail for non-existent session"
    );
    let err = result.unwrap_err();
    let err_string = format!("{err}");
    assert!(
        err_string.contains("not found") || err_string.contains("WorktreeNotFound"),
        "Error should indicate worktree not found: {err_string}"
    );
}

// =============================================================================
// Scenario: Discard orphaned session removes worktree
// =============================================================================

/// Scenario: Discard orphaned session removes worktree
///
/// @step Given a git repository with an initial commit
/// @step And an orphaned session worktree
/// @step When I call discard_session with the session ID
/// @step Then the session worktree should be removed
/// @step And the DiscardResult should have previous_status equal to Orphaned
#[test]
fn test_discard_orphaned_session() {
    let session_id = unique_session_id("discard_orphaned");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And an orphaned session worktree
    // Create worktree but NO manifest (orphaned = no manifest)
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // Explicitly NOT creating manifest - making it orphaned
    // The worktree exists but no manifest = Orphaned status

    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");

    // @step When I call discard_session with the session ID
    let result =
        discard_session(repo_path, &session_id).expect("Failed to discard orphaned session");

    // @step Then the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after discard"
    );

    // @step And the DiscardResult should have previous_status equal to Orphaned
    assert_eq!(
        result.previous_status,
        DerivedSessionStatus::Orphaned,
        "previous_status should be Orphaned, got: {:?}",
        result.previous_status
    );
}

// =============================================================================
// Scenario: Discard session cleans up manifest
// =============================================================================

/// Scenario: Discard session cleans up manifest
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with a manifest in ~/.fspec/git-sessions/
/// @step When I call discard_session with the session ID
/// @step Then the session worktree should be removed
/// @step And the session manifest should be deleted from ~/.fspec/git-sessions/
#[test]
fn test_discard_session_cleans_up_manifest() {
    let session_id = unique_session_id("discard_manifest");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree with a manifest in ~/.fspec/git-sessions/
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    // Verify manifest exists
    let manifest_path = get_manifest_path(&session_id).expect("Should have manifest path");
    assert!(
        manifest_path.exists(),
        "Manifest should exist before discard"
    );

    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");

    // @step When I call discard_session with the session ID
    let _result = discard_session(repo_path, &session_id).expect("Failed to discard session");

    // @step Then the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after discard"
    );

    // @step And the session manifest should be deleted from ~/.fspec/git-sessions/
    assert!(
        !manifest_path.exists(),
        "Manifest should be deleted after discard"
    );
}
