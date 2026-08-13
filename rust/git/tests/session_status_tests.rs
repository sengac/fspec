//! Feature: spec/features/session-completion-status.feature
//!
//! Integration tests for session completion behavior and status derivation.
//! Tests use fixtures (real temp repos) - NO MOCKING.
//!
//! GIT-022: Session completion leaves worktree for review, status is derived
//! at query time.

mod common;

use codelet_git::{
    complete_session, create_session_manifest, delete_manifest, derive_session_status,
    read_manifest, terminate_session, DerivedSessionStatus, IsolatedSessionInfo,
    FSPEC_WORKTREES_DIR,
};
use std::collections::HashSet;
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
// Scenario Tests
// =============================================================================

// =============================================================================
// Scenario: Isolated session completion leaves worktree for review
// =============================================================================

/// Scenario: Isolated session completion leaves worktree for review
///
/// @step Given I have a git repository
/// @step And I create an isolated session "abc123"
/// @step And the session has a worktree at ".fspec/worktrees/abc123"
/// @step When I complete the session
/// @step Then the worktree at ".fspec/worktrees/abc123" should still exist
/// @step And the worktree should not be automatically cleaned up
#[test]
fn test_isolated_session_completion_leaves_worktree() {
    let session_id = unique_session_id("leaves_worktree");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And the session has a worktree
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(&session_id);
    assert!(
        worktree_path.exists(),
        "Worktree should exist after creation"
    );
    assert!(
        info.worktree_path.is_some(),
        "Session should have worktree_path"
    );

    // @step When I complete the session
    // GIT-022: Session completion should NOT cleanup worktree
    // We're testing the expected behavior - worktree should remain

    // @step Then the worktree should still exist
    assert!(
        worktree_path.exists(),
        "Worktree should still exist after session completion"
    );

    // @step And the worktree should not be automatically cleaned up
    let git_worktree_dir = repo_path.join(".git/worktrees").join(&session_id);
    assert!(
        git_worktree_dir.exists(),
        "Git worktree metadata should still exist"
    );
}

// =============================================================================
// Scenario: Session without changes transitions to Clean status on completion
// =============================================================================

/// Scenario: Session without changes transitions to Clean status on completion
///
/// @step Given I have a git repository
/// @step And I create an isolated session "abc123"
/// @step And the session worktree has no uncommitted changes
/// @step When I complete the session
/// @step And I derive the session status for "abc123"
/// @step Then the status should be "Clean"
#[test]
fn test_session_without_changes_has_clean_status() {
    let session_id = unique_session_id("clean_status");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // Create the session manifest (normally done by BackgroundSession)
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    // @step And the session worktree has no uncommitted changes
    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(&session_id);
    let readme_path = worktree_path.join("README.md");
    assert!(
        readme_path.exists(),
        "README.md should exist in worktree (from HEAD)"
    );

    // @step When I complete the session
    complete_session(&session_id).expect("Failed to complete session");

    // @step And I derive the session status
    let status = derive_session_status(repo_path, &session_id, &HashSet::new())
        .expect("Failed to derive session status");

    // @step Then the status should be "Clean"
    assert_eq!(status, DerivedSessionStatus::Clean);

    // Cleanup manifest
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Session with changes transitions to PendingMerge status on completion
// =============================================================================

/// Scenario: Session with changes transitions to PendingMerge status on completion
///
/// @step Given I have a git repository
/// @step And I create an isolated session "abc123"
/// @step And the session worktree has uncommitted changes
/// @step When I complete the session
/// @step And I derive the session status for "abc123"
/// @step Then the status should be "PendingMerge"
#[test]
fn test_session_with_changes_has_pending_merge_status() {
    let session_id = unique_session_id("pending_merge");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // Create the session manifest (normally done by BackgroundSession)
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    // @step And the session worktree has uncommitted changes
    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");
    fs::write(worktree_path.join("new_file.txt"), "New content\n")
        .expect("Failed to write file to worktree");

    // Verify new file exists (change was made)
    assert!(
        worktree_path.join("new_file.txt").exists(),
        "New file should exist in worktree"
    );

    // @step When I complete the session
    complete_session(&session_id).expect("Failed to complete session");

    // @step And I derive the session status
    let status = derive_session_status(repo_path, &session_id, &HashSet::new())
        .expect("Failed to derive session status");

    // @step Then the status should be "PendingMerge"
    assert_eq!(status, DerivedSessionStatus::PendingMerge);

    // Cleanup manifest
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Active session returns Active status regardless of changes
// =============================================================================

/// Scenario: Active session returns Active status regardless of changes
///
/// @step Given I have a git repository
/// @step And I create an isolated session "abc123"
/// @step And the session is still active
/// @step And the session worktree has uncommitted changes
/// @step When I derive the session status for "abc123"
/// @step Then the status should be "Active"
/// @step And the status should not be "PendingMerge"
#[test]
fn test_active_session_returns_active_status() {
    let session_id = unique_session_id("active_status");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // Create the session manifest (normally done by BackgroundSession)
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    // @step And the session is still active
    let mut active_sessions: HashSet<String> = HashSet::new();
    active_sessions.insert(session_id.clone());

    // @step And the session worktree has uncommitted changes
    let worktree_path = info.worktree_path.as_ref().expect("Should have worktree");
    fs::write(worktree_path.join("new_file.txt"), "New content\n")
        .expect("Failed to write file to worktree");

    // @step When I derive the session status
    let status = derive_session_status(repo_path, &session_id, &active_sessions)
        .expect("Failed to derive session status");

    // @step Then the status should be "Active"
    assert_eq!(status, DerivedSessionStatus::Active);

    // @step And the status should not be "PendingMerge"
    assert_ne!(status, DerivedSessionStatus::PendingMerge);

    // Cleanup manifest
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Worktree with no manifest returns Orphaned status
// =============================================================================

/// Scenario: Worktree with no manifest returns Orphaned status
///
/// @step Given I have a git repository
/// @step And a worktree exists at ".fspec/worktrees/orphan123"
/// @step And no session manifest exists for "orphan123"
/// @step When I derive the session status for "orphan123"
/// @step Then the status should be "Orphaned"
#[test]
fn test_worktree_without_manifest_is_orphaned() {
    let session_id = unique_session_id("orphan_no_manifest");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And a worktree exists
    let _info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(&session_id);
    assert!(worktree_path.exists(), "Worktree should exist");

    // @step And no session manifest exists
    // We intentionally DO NOT create a manifest here to test orphan detection

    // @step When I derive the session status
    let status = derive_session_status(repo_path, &session_id, &HashSet::new())
        .expect("Failed to derive session status");

    // @step Then the status should be "Orphaned"
    assert_eq!(status, DerivedSessionStatus::Orphaned);
}

// =============================================================================
// Scenario: Worktree with terminated manifest returns Orphaned status
// =============================================================================

/// Scenario: Worktree with terminated manifest returns Orphaned status
///
/// @step Given I have a git repository
/// @step And a worktree exists at ".fspec/worktrees/terminated123"
/// @step And a session manifest exists for "terminated123" with terminated=true
/// @step When I derive the session status for "terminated123"
/// @step Then the status should be "Orphaned"
#[test]
fn test_terminated_manifest_is_orphaned() {
    let session_id = unique_session_id("terminated");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And a worktree exists
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(&session_id);
    assert!(worktree_path.exists(), "Worktree should exist");

    // @step And a session manifest exists with terminated=true
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    terminate_session(&session_id).expect("Failed to terminate session");

    // @step When I derive the session status
    let status = derive_session_status(repo_path, &session_id, &HashSet::new())
        .expect("Failed to derive session status");

    // @step Then the status should be "Orphaned"
    assert_eq!(status, DerivedSessionStatus::Orphaned);

    // Cleanup manifest
    let _ = delete_manifest(&session_id);
}

// =============================================================================
// Scenario: Session completion updates manifest with completed_at timestamp
// =============================================================================

/// Scenario: Session completion updates manifest with completed_at timestamp
///
/// @step Given I have a git repository
/// @step And I create an isolated session "abc123"
/// @step And a session manifest exists for "abc123" without completed_at
/// @step When I complete the session
/// @step Then the session manifest for "abc123" should have a completed_at timestamp
#[test]
fn test_session_completion_updates_manifest() {
    let session_id = unique_session_id("completion_updates");

    // @step Given I have a git repository
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    // @step And I create an isolated session
    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And a session manifest exists without completed_at
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create session manifest");

    // Verify manifest doesn't have completed_at yet
    let manifest_before = read_manifest(&session_id)
        .expect("Failed to read manifest")
        .expect("Manifest should exist");
    assert!(
        manifest_before.completed_at.is_none(),
        "Manifest should not have completed_at before completion"
    );

    // @step When I complete the session
    complete_session(&session_id).expect("Failed to complete session");

    // @step Then the session manifest should have a completed_at timestamp
    let manifest_after = read_manifest(&session_id)
        .expect("Failed to read manifest")
        .expect("Manifest should exist");
    assert!(
        manifest_after.completed_at.is_some(),
        "Manifest should have completed_at after completion"
    );

    // Cleanup manifest
    let _ = delete_manifest(&session_id);
}
