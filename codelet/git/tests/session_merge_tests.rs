//! Feature: spec/features/session-merge-operations.feature
//!
//! Integration tests for session merge operations.
//! Tests use fixtures (real temp repos) - NO MOCKING.
//!
//! GIT-024: Merge session changes to main worktree with conflict detection.

mod common;

use codelet_git::{create_session_manifest, delete_manifest, merge_session, IsolatedSessionInfo};
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
// Scenario: Merge session changes to main worktree
// =============================================================================

/// Scenario: Merge session changes to main worktree
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with a modified file "src/main.rs"
/// @step When I call merge_session with the session ID
/// @step Then the modified file should be updated in the main worktree
/// @step And the session worktree should be removed
/// @step And the MergeResult should contain "src/main.rs" in files_modified
#[test]
fn test_merge_session_changes_to_main() {
    let session_id = unique_session_id("merge_main");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree with a modified file "src/main.rs"
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // Create manifest
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
        "fn main() { println!(\"Hello\"); }\n",
    )
    .expect("Failed to modify main.rs in worktree");

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id).expect("Failed to merge session");

    // @step Then the modified file should be updated in the main worktree
    let main_content = fs::read_to_string(repo_path.join("src/main.rs"))
        .expect("Failed to read main.rs from main worktree");
    assert!(
        main_content.contains("println"),
        "Main worktree should have modified content"
    );

    // @step And the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after merge"
    );

    // @step And the MergeResult should contain "src/main.rs" in files_modified
    assert!(
        result.files_modified.contains(&"src/main.rs".to_string()),
        "MergeResult should contain src/main.rs in files_modified: {:?}",
        result.files_modified
    );
}

// =============================================================================
// Scenario: Merge session applies added files
// =============================================================================

/// Scenario: Merge session applies added files
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with a new file "src/new.rs"
/// @step When I call merge_session with the session ID
/// @step Then the new file should exist in the main worktree
/// @step And the session worktree should be removed
/// @step And the MergeResult should contain "src/new.rs" in files_added
#[test]
fn test_merge_session_applies_added_files() {
    let session_id = unique_session_id("merge_added");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree with a new file "src/new.rs"
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

    // Add new file in session worktree
    fs::write(
        worktree_path.join("src/new.rs"),
        "// New file\npub fn new_func() {}\n",
    )
    .expect("Failed to create new.rs in worktree");

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id).expect("Failed to merge session");

    // @step Then the new file should exist in the main worktree
    assert!(
        repo_path.join("src/new.rs").exists(),
        "New file should exist in main worktree"
    );

    // @step And the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after merge"
    );

    // @step And the MergeResult should contain "src/new.rs" in files_added
    assert!(
        result.files_added.contains(&"src/new.rs".to_string()),
        "MergeResult should contain src/new.rs in files_added: {:?}",
        result.files_added
    );
}

// =============================================================================
// Scenario: Merge session applies deleted files
// =============================================================================

/// Scenario: Merge session applies deleted files
///
/// @step Given a git repository with an initial commit containing "src/old.rs"
/// @step And a session worktree where "src/old.rs" has been deleted
/// @step When I call merge_session with the session ID
/// @step Then "src/old.rs" should not exist in the main worktree
/// @step And the session worktree should be removed
/// @step And the MergeResult should contain "src/old.rs" in files_deleted
#[test]
fn test_merge_session_applies_deleted_files() {
    let session_id = unique_session_id("merge_deleted");

    // @step Given a git repository with an initial commit containing "src/old.rs"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // Verify src/old.rs exists in base
    assert!(
        repo_path.join("src/old.rs").exists(),
        "src/old.rs should exist in base"
    );

    // @step And a session worktree where "src/old.rs" has been deleted
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

    // Delete src/old.rs in session worktree
    fs::remove_file(worktree_path.join("src/old.rs")).expect("Failed to delete old.rs in worktree");

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id).expect("Failed to merge session");

    // @step Then "src/old.rs" should not exist in the main worktree
    assert!(
        !repo_path.join("src/old.rs").exists(),
        "src/old.rs should be deleted from main worktree"
    );

    // @step And the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after merge"
    );

    // @step And the MergeResult should contain "src/old.rs" in files_deleted
    assert!(
        result.files_deleted.contains(&"src/old.rs".to_string()),
        "MergeResult should contain src/old.rs in files_deleted: {:?}",
        result.files_deleted
    );
}

// =============================================================================
// Scenario: Merge session fails when main has conflicting changes
// =============================================================================

/// Scenario: Merge session fails when main has conflicting changes
///
/// @step Given a git repository with an initial commit containing "src/config.rs"
/// @step And a session worktree where "src/config.rs" has been modified
/// @step And "src/config.rs" has also been modified in the main worktree
/// @step When I call merge_session with the session ID
/// @step Then a ConflictError should be returned
/// @step And the ConflictError should list "src/config.rs" as a conflicting file
/// @step And the session worktree should still exist
#[test]
fn test_merge_session_fails_on_conflict() {
    let session_id = unique_session_id("merge_conflict");

    // @step Given a git repository with an initial commit containing "src/config.rs"
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree where "src/config.rs" has been modified
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

    // Modify src/config.rs in session worktree
    fs::write(
        worktree_path.join("src/config.rs"),
        "// Session changes\nconst A: i32 = 1;\n",
    )
    .expect("Failed to modify config.rs in worktree");

    // @step And "src/config.rs" has also been modified in the main worktree
    fs::write(
        repo_path.join("src/config.rs"),
        "// Main changes\nconst B: i32 = 2;\n",
    )
    .expect("Failed to modify config.rs in main");

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id);

    // @step Then a ConflictError should be returned
    assert!(result.is_err(), "merge_session should fail with conflict");
    let err = result.unwrap_err();
    let err_string = format!("{err}");

    // @step And the ConflictError should list "src/config.rs" as a conflicting file
    assert!(
        err_string.contains("config.rs") || err_string.contains("conflict"),
        "Error should mention conflicting file: {err_string}"
    );

    // @step And the session worktree should still exist
    assert!(
        worktree_path.exists(),
        "Session worktree should still exist after conflict"
    );

    // Cleanup
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Merge session fails when added file conflicts with main
// =============================================================================

/// Scenario: Merge session fails when added file conflicts with main
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with a new file "src/feature.rs" containing "session content"
/// @step And the main worktree also has "src/feature.rs" with different content
/// @step When I call merge_session with the session ID
/// @step Then a ConflictError should be returned
/// @step And the ConflictError should list "src/feature.rs" as a conflicting file
/// @step And the session worktree should still exist
#[test]
fn test_merge_session_fails_on_added_file_conflict() {
    let session_id = unique_session_id("merge_add_conflict");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And a session worktree with a new file "src/feature.rs" containing "session content"
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

    // Add new file in session worktree
    fs::write(worktree_path.join("src/feature.rs"), "// Session content\n")
        .expect("Failed to create feature.rs in worktree");

    // @step And the main worktree also has "src/feature.rs" with different content
    fs::write(
        repo_path.join("src/feature.rs"),
        "// Main content - different!\n",
    )
    .expect("Failed to create feature.rs in main");

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id);

    // @step Then a ConflictError should be returned
    assert!(result.is_err(), "merge_session should fail with conflict");
    let err = result.unwrap_err();
    let err_string = format!("{err}");

    // @step And the ConflictError should list "src/feature.rs" as a conflicting file
    assert!(
        err_string.contains("feature.rs") || err_string.contains("conflict"),
        "Error should mention conflicting file: {err_string}"
    );

    // @step And the session worktree should still exist
    assert!(
        worktree_path.exists(),
        "Session worktree should still exist after conflict"
    );

    // Cleanup
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Merge multiple pending sessions in chosen order
// =============================================================================

/// Scenario: Merge multiple pending sessions in chosen order
///
/// @step Given a git repository with an initial commit
/// @step And three session worktrees "session-A", "session-B", "session-C" each with different changes
/// @step When I merge sessions in order: "session-B", "session-A", "session-C"
/// @step Then all merges should succeed
/// @step And all session worktrees should be removed
/// @step And the main worktree should contain changes from all sessions
#[test]
fn test_merge_multiple_sessions_in_order() {
    let session_a = unique_session_id("multi_A");
    let session_b = unique_session_id("multi_B");
    let session_c = unique_session_id("multi_C");

    // @step Given a git repository with an initial commit
    let tmp_dir = common::setup_test_repo_with_files();
    let repo_path = tmp_dir.path();

    // @step And three session worktrees "session-A", "session-B", "session-C" each with different changes
    let info_a = IsolatedSessionInfo::new_isolated(repo_path, &session_a)
        .expect("Failed to create session A");
    let info_b = IsolatedSessionInfo::new_isolated(repo_path, &session_b)
        .expect("Failed to create session B");
    let info_c = IsolatedSessionInfo::new_isolated(repo_path, &session_c)
        .expect("Failed to create session C");

    // Create manifests
    for (id, info) in [
        (&session_a, &info_a),
        (&session_b, &info_b),
        (&session_c, &info_c),
    ] {
        create_session_manifest(
            id,
            repo_path,
            info.worktree_path.clone(),
            info.base_commit.clone(),
        )
        .expect("Failed to create session manifest");
    }

    // Add different files to each session
    let wt_a = info_a
        .worktree_path
        .as_ref()
        .expect("Should have worktree A");
    let wt_b = info_b
        .worktree_path
        .as_ref()
        .expect("Should have worktree B");
    let wt_c = info_c
        .worktree_path
        .as_ref()
        .expect("Should have worktree C");

    fs::write(wt_a.join("file_a.txt"), "Content from A\n")
        .expect("Failed to create file in session A");
    fs::write(wt_b.join("file_b.txt"), "Content from B\n")
        .expect("Failed to create file in session B");
    fs::write(wt_c.join("file_c.txt"), "Content from C\n")
        .expect("Failed to create file in session C");

    // @step When I merge sessions in order: "session-B", "session-A", "session-C"
    let result_b = merge_session(repo_path, &session_b);
    let result_a = merge_session(repo_path, &session_a);
    let result_c = merge_session(repo_path, &session_c);

    // @step Then all merges should succeed
    assert!(
        result_b.is_ok(),
        "Session B merge should succeed: {:?}",
        result_b.err()
    );
    assert!(
        result_a.is_ok(),
        "Session A merge should succeed: {:?}",
        result_a.err()
    );
    assert!(
        result_c.is_ok(),
        "Session C merge should succeed: {:?}",
        result_c.err()
    );

    // @step And all session worktrees should be removed
    assert!(!wt_a.exists(), "Session A worktree should be removed");
    assert!(!wt_b.exists(), "Session B worktree should be removed");
    assert!(!wt_c.exists(), "Session C worktree should be removed");

    // @step And the main worktree should contain changes from all sessions
    assert!(
        repo_path.join("file_a.txt").exists(),
        "file_a.txt should exist in main"
    );
    assert!(
        repo_path.join("file_b.txt").exists(),
        "file_b.txt should exist in main"
    );
    assert!(
        repo_path.join("file_c.txt").exists(),
        "file_c.txt should exist in main"
    );
}

// =============================================================================
// Scenario: Merge clean session removes worktree
// =============================================================================

/// Scenario: Merge clean session removes worktree
///
/// @step Given a git repository with an initial commit
/// @step And a session worktree with no changes
/// @step When I call merge_session with the session ID
/// @step Then the session worktree should be removed
/// @step And the MergeResult should have empty files_modified
/// @step And the MergeResult should have empty files_added
/// @step And the MergeResult should have empty files_deleted
#[test]
fn test_merge_clean_session_removes_worktree() {
    let session_id = unique_session_id("merge_clean");

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

    // @step When I call merge_session with the session ID
    let result = merge_session(repo_path, &session_id).expect("Failed to merge clean session");

    // @step Then the session worktree should be removed
    assert!(
        !worktree_path.exists(),
        "Session worktree should be removed after merge"
    );

    // @step And the MergeResult should have empty files_modified
    assert!(
        result.files_modified.is_empty(),
        "files_modified should be empty: {:?}",
        result.files_modified
    );

    // @step And the MergeResult should have empty files_added
    assert!(
        result.files_added.is_empty(),
        "files_added should be empty: {:?}",
        result.files_added
    );

    // @step And the MergeResult should have empty files_deleted
    assert!(
        result.files_deleted.is_empty(),
        "files_deleted should be empty: {:?}",
        result.files_deleted
    );
}
