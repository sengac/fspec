//! Feature: spec/features/session-orphan-pruning.feature
//!
//! Integration tests for session orphan detection and pruning.
//! Tests use fixtures (real temp repos) - NO MOCKING.
//!
//! GIT-026: Orphan detection and pruning.

mod common;

use codelet_git::{
    create_session_manifest, delete_manifest, get_manifest_path, is_orphaned, prune_orphaned,
    read_manifest, terminate_session, IsolatedSessionInfo,
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
// ORPHAN DETECTION SCENARIOS
// =============================================================================

// -----------------------------------------------------------------------------
// Scenario: Detect orphaned worktree when session manifest is missing
// -----------------------------------------------------------------------------

/// Scenario: Detect orphaned worktree when session manifest is missing
///
/// @step Given a git repository with an isolated session worktree "session-1"
/// @step And no session manifest exists for "session-1"
/// @step And "session-1" is not in the active sessions set
/// @step When I check if "session-1" is orphaned
/// @step Then the session should be detected as orphaned
#[test]
fn test_detect_orphaned_when_manifest_missing() {
    let session_id = unique_session_id("orphan_no_manifest");

    // @step Given a git repository with an isolated session worktree "session-1"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let _info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And no session manifest exists for "session-1"
    // NOTE: We deliberately do NOT create a manifest
    if let Some(manifest_path) = get_manifest_path(&session_id) {
        // Ensure no manifest exists (should not exist anyway)
        let _ = fs::remove_file(&manifest_path);
        assert!(!manifest_path.exists(), "Manifest should not exist");
    }

    // @step And "session-1" is not in the active sessions set
    let active_sessions: HashSet<String> = HashSet::new();

    // @step When I check if "session-1" is orphaned
    let result = is_orphaned(&session_id, &active_sessions).expect("is_orphaned should not fail");

    // @step Then the session should be detected as orphaned
    assert!(
        result,
        "Session without manifest should be detected as orphaned"
    );
}

// -----------------------------------------------------------------------------
// Scenario: Detect orphaned worktree when session manifest is terminated
// -----------------------------------------------------------------------------

/// Scenario: Detect orphaned worktree when session manifest is terminated
///
/// @step Given a git repository with an isolated session worktree "session-2"
/// @step And a session manifest exists for "session-2" with terminated flag set to true
/// @step And "session-2" is not in the active sessions set
/// @step When I check if "session-2" is orphaned
/// @step Then the session should be detected as orphaned
#[test]
fn test_detect_orphaned_when_manifest_terminated() {
    let session_id = unique_session_id("orphan_terminated");

    // @step Given a git repository with an isolated session worktree "session-2"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And a session manifest exists for "session-2" with terminated flag set to true
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create manifest");

    // Mark as terminated
    terminate_session(&session_id).expect("Failed to terminate session");

    // Verify manifest has terminated=true
    let manifest = read_manifest(&session_id)
        .expect("Should read manifest")
        .expect("Manifest should exist");
    assert!(manifest.terminated, "Manifest should have terminated=true");

    // @step And "session-2" is not in the active sessions set
    let active_sessions: HashSet<String> = HashSet::new();

    // @step When I check if "session-2" is orphaned
    let result = is_orphaned(&session_id, &active_sessions).expect("is_orphaned should not fail");

    // @step Then the session should be detected as orphaned
    assert!(
        result,
        "Session with terminated manifest should be detected as orphaned"
    );
}

// -----------------------------------------------------------------------------
// Scenario: Active session with missing manifest is not orphaned
// -----------------------------------------------------------------------------

/// Scenario: Active session with missing manifest is not orphaned
///
/// @step Given a git repository with an isolated session worktree "session-3"
/// @step And no session manifest exists for "session-3"
/// @step And "session-3" is in the active sessions set
/// @step When I check if "session-3" is orphaned
/// @step Then the session should NOT be detected as orphaned
#[test]
fn test_active_session_not_orphaned_even_without_manifest() {
    let session_id = unique_session_id("active_no_manifest");

    // @step Given a git repository with an isolated session worktree "session-3"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let _info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And no session manifest exists for "session-3"
    // NOTE: We deliberately do NOT create a manifest
    if let Some(manifest_path) = get_manifest_path(&session_id) {
        let _ = fs::remove_file(&manifest_path);
    }

    // @step And "session-3" is in the active sessions set
    let mut active_sessions: HashSet<String> = HashSet::new();
    active_sessions.insert(session_id.clone());

    // @step When I check if "session-3" is orphaned
    let result = is_orphaned(&session_id, &active_sessions).expect("is_orphaned should not fail");

    // @step Then the session should NOT be detected as orphaned
    assert!(
        !result,
        "Active session should NOT be detected as orphaned, regardless of manifest state"
    );
}

// -----------------------------------------------------------------------------
// Scenario: Session with valid non-terminated manifest is not orphaned
// -----------------------------------------------------------------------------

/// Scenario: Session with valid non-terminated manifest is not orphaned
///
/// @step Given a git repository with an isolated session worktree "session-4"
/// @step And a session manifest exists for "session-4" with terminated flag set to false
/// @step And "session-4" is not in the active sessions set
/// @step When I check if "session-4" is orphaned
/// @step Then the session should NOT be detected as orphaned
#[test]
fn test_session_with_valid_manifest_not_orphaned() {
    let session_id = unique_session_id("valid_manifest");

    // @step Given a git repository with an isolated session worktree "session-4"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let info = IsolatedSessionInfo::new_isolated(repo_path, &session_id)
        .expect("Failed to create isolated session");

    // @step And a session manifest exists for "session-4" with terminated flag set to false
    create_session_manifest(
        &session_id,
        repo_path,
        info.worktree_path.clone(),
        info.base_commit.clone(),
    )
    .expect("Failed to create manifest");

    // Verify manifest has terminated=false (default)
    let manifest = read_manifest(&session_id)
        .expect("Should read manifest")
        .expect("Manifest should exist");
    assert!(!manifest.terminated, "Manifest should have terminated=false");

    // @step And "session-4" is not in the active sessions set
    let active_sessions: HashSet<String> = HashSet::new();

    // @step When I check if "session-4" is orphaned
    let result = is_orphaned(&session_id, &active_sessions).expect("is_orphaned should not fail");

    // @step Then the session should NOT be detected as orphaned
    assert!(
        !result,
        "Session with valid non-terminated manifest should NOT be detected as orphaned"
    );
}

// =============================================================================
// PRUNE SCENARIOS
// =============================================================================

// -----------------------------------------------------------------------------
// Scenario: Prune all orphaned worktrees
// -----------------------------------------------------------------------------

/// Scenario: Prune all orphaned worktrees
///
/// @step Given a git repository with 3 isolated session worktrees
/// @step And all 3 sessions have no manifest files
/// @step And none of the sessions are active
/// @step When I prune orphaned worktrees
/// @step Then the result should indicate 3 worktrees were pruned
/// @step And the result should contain all 3 session IDs
/// @step And all worktree directories should be removed
/// @step And all session manifest files should be cleaned up
#[test]
fn test_prune_all_orphaned_worktrees() {
    let session_1 = unique_session_id("prune_orphan_1");
    let session_2 = unique_session_id("prune_orphan_2");
    let session_3 = unique_session_id("prune_orphan_3");

    // @step Given a git repository with 3 isolated session worktrees
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let info_1 = IsolatedSessionInfo::new_isolated(repo_path, &session_1)
        .expect("Failed to create session 1");
    let info_2 = IsolatedSessionInfo::new_isolated(repo_path, &session_2)
        .expect("Failed to create session 2");
    let info_3 = IsolatedSessionInfo::new_isolated(repo_path, &session_3)
        .expect("Failed to create session 3");

    let worktree_1 = info_1.worktree_path.clone().unwrap();
    let worktree_2 = info_2.worktree_path.clone().unwrap();
    let worktree_3 = info_3.worktree_path.clone().unwrap();

    // Verify worktrees exist
    assert!(worktree_1.exists(), "Worktree 1 should exist");
    assert!(worktree_2.exists(), "Worktree 2 should exist");
    assert!(worktree_3.exists(), "Worktree 3 should exist");

    // @step And all 3 sessions have no manifest files
    // NOTE: We don't create manifests, making them orphaned

    // @step And none of the sessions are active
    let active_sessions: HashSet<String> = HashSet::new();

    // @step When I prune orphaned worktrees
    let result =
        prune_orphaned(repo_path, &active_sessions).expect("prune_orphaned should not fail");

    // @step Then the result should indicate 3 worktrees were pruned
    assert_eq!(
        result.count, 3,
        "Should have pruned 3 worktrees, got: {}",
        result.count
    );

    // @step And the result should contain all 3 session IDs
    assert!(
        result.pruned.contains(&session_1),
        "Pruned list should contain session_1"
    );
    assert!(
        result.pruned.contains(&session_2),
        "Pruned list should contain session_2"
    );
    assert!(
        result.pruned.contains(&session_3),
        "Pruned list should contain session_3"
    );

    // @step And all worktree directories should be removed
    assert!(
        !worktree_1.exists(),
        "Worktree 1 should be removed after prune"
    );
    assert!(
        !worktree_2.exists(),
        "Worktree 2 should be removed after prune"
    );
    assert!(
        !worktree_3.exists(),
        "Worktree 3 should be removed after prune"
    );

    // @step And all session manifest files should be cleaned up
    // (They didn't exist to begin with, but verify cleanup behavior)
    if let Some(path) = get_manifest_path(&session_1) {
        assert!(!path.exists(), "Manifest 1 should not exist");
    }
    if let Some(path) = get_manifest_path(&session_2) {
        assert!(!path.exists(), "Manifest 2 should not exist");
    }
    if let Some(path) = get_manifest_path(&session_3) {
        assert!(!path.exists(), "Manifest 3 should not exist");
    }
}

// -----------------------------------------------------------------------------
// Scenario: Prune returns zero when no orphaned worktrees exist
// -----------------------------------------------------------------------------

/// Scenario: Prune returns zero when no orphaned worktrees exist
///
/// @step Given a git repository with 2 isolated session worktrees
/// @step And all sessions have valid non-terminated manifest files
/// @step When I prune orphaned worktrees
/// @step Then the result should indicate 0 worktrees were pruned
/// @step And the result should contain an empty list of pruned session IDs
/// @step And all worktree directories should still exist
#[test]
fn test_prune_returns_zero_when_no_orphans() {
    let session_1 = unique_session_id("prune_valid_1");
    let session_2 = unique_session_id("prune_valid_2");

    // @step Given a git repository with 2 isolated session worktrees
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let info_1 = IsolatedSessionInfo::new_isolated(repo_path, &session_1)
        .expect("Failed to create session 1");
    let info_2 = IsolatedSessionInfo::new_isolated(repo_path, &session_2)
        .expect("Failed to create session 2");

    let worktree_1 = info_1.worktree_path.clone().unwrap();
    let worktree_2 = info_2.worktree_path.clone().unwrap();

    // @step And all sessions have valid non-terminated manifest files
    create_session_manifest(
        &session_1,
        repo_path,
        info_1.worktree_path.clone(),
        info_1.base_commit.clone(),
    )
    .expect("Failed to create manifest 1");

    create_session_manifest(
        &session_2,
        repo_path,
        info_2.worktree_path.clone(),
        info_2.base_commit.clone(),
    )
    .expect("Failed to create manifest 2");

    let active_sessions: HashSet<String> = HashSet::new();

    // @step When I prune orphaned worktrees
    let result =
        prune_orphaned(repo_path, &active_sessions).expect("prune_orphaned should not fail");

    // @step Then the result should indicate 0 worktrees were pruned
    assert_eq!(
        result.count, 0,
        "Should have pruned 0 worktrees, got: {}",
        result.count
    );

    // @step And the result should contain an empty list of pruned session IDs
    assert!(
        result.pruned.is_empty(),
        "Pruned list should be empty, got: {:?}",
        result.pruned
    );

    // @step And all worktree directories should still exist
    assert!(
        worktree_1.exists(),
        "Worktree 1 should still exist after prune"
    );
    assert!(
        worktree_2.exists(),
        "Worktree 2 should still exist after prune"
    );

    // Cleanup manifests
    let _ = delete_manifest(&session_1);
    let _ = delete_manifest(&session_2);
}

// -----------------------------------------------------------------------------
// Scenario: Prune returns list of pruned session IDs
// -----------------------------------------------------------------------------

/// Scenario: Prune returns list of pruned session IDs
///
/// @step Given a git repository with 2 orphaned session worktrees "orphan-1" and "orphan-2"
/// @step And 1 active session worktree "active-1"
/// @step When I prune orphaned worktrees
/// @step Then the result should indicate 2 worktrees were pruned
/// @step And the result should contain session IDs "orphan-1" and "orphan-2"
/// @step And the result should NOT contain session ID "active-1"
#[test]
fn test_prune_returns_list_of_pruned_ids() {
    let orphan_1 = unique_session_id("orphan_prune_1");
    let orphan_2 = unique_session_id("orphan_prune_2");
    let active_1 = unique_session_id("active_prune_1");

    // @step Given a git repository with 2 orphaned session worktrees "orphan-1" and "orphan-2"
    let tmp_dir = common::setup_test_repo();
    let repo_path = tmp_dir.path();

    let _info_orphan_1 = IsolatedSessionInfo::new_isolated(repo_path, &orphan_1)
        .expect("Failed to create orphan 1");
    let _info_orphan_2 = IsolatedSessionInfo::new_isolated(repo_path, &orphan_2)
        .expect("Failed to create orphan 2");

    // @step And 1 active session worktree "active-1"
    let info_active = IsolatedSessionInfo::new_isolated(repo_path, &active_1)
        .expect("Failed to create active session");

    // Create manifest for active session (so it's not orphaned due to valid manifest)
    create_session_manifest(
        &active_1,
        repo_path,
        info_active.worktree_path.clone(),
        info_active.base_commit.clone(),
    )
    .expect("Failed to create manifest for active session");

    // Also mark active session as active
    let mut active_sessions: HashSet<String> = HashSet::new();
    active_sessions.insert(active_1.clone());

    // @step When I prune orphaned worktrees
    let result =
        prune_orphaned(repo_path, &active_sessions).expect("prune_orphaned should not fail");

    // @step Then the result should indicate 2 worktrees were pruned
    assert_eq!(
        result.count, 2,
        "Should have pruned 2 orphaned worktrees, got: {}",
        result.count
    );

    // @step And the result should contain session IDs "orphan-1" and "orphan-2"
    assert!(
        result.pruned.contains(&orphan_1),
        "Pruned list should contain orphan_1"
    );
    assert!(
        result.pruned.contains(&orphan_2),
        "Pruned list should contain orphan_2"
    );

    // @step And the result should NOT contain session ID "active-1"
    assert!(
        !result.pruned.contains(&active_1),
        "Pruned list should NOT contain active session"
    );

    // Verify active session worktree still exists
    assert!(
        info_active.worktree_path.as_ref().unwrap().exists(),
        "Active session worktree should still exist"
    );

    // Cleanup
    let _ = delete_manifest(&active_1);
}
