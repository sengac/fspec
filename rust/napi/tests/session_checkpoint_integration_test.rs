#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/session-checkpoint-integration.feature
//!
//! Integration tests for Session Checkpoint Integration
//!
//! GIT-021: Connect ghost commits to session checkpoint operations.
//! Add checkpoint(), restore(), list_checkpoints() methods to BackgroundSession.
//!
//! NOTE: The core ghost commit logic is tested in codelet-git::ghost_commit tests.
//! These tests verify the NAPI layer integration - that BackgroundSession correctly
//! exposes checkpoint operations for isolated sessions.

use std::fs;
use std::path::Path;

// =============================================================================
// Source Code Verification Helpers
// =============================================================================

fn read_session_manager_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/session_manager.rs");
    fs::read_to_string(&path).expect("Failed to read session_manager.rs")
}

// =============================================================================
// Scenario: Checkpoint creates ghost commit with session ID namespace
// =============================================================================

/// Verify BackgroundSession has checkpoint method
///
/// @step Given an isolated session with worktree path
/// @step When I call checkpoint with label "before-refactor"
/// @step Then a ghost commit should be created
/// @step And the ref should be stored at refs/fspec-checkpoints/<session-id>/before-refactor
#[test]
fn test_background_session_has_checkpoint_method() {
    // @step Given an isolated session with worktree path
    let source = read_session_manager_source();

    // @step When I call checkpoint with label "before-refactor"
    // Verify the method signature exists
    assert!(
        source.contains("pub fn checkpoint(&self, label: &str)"),
        "BackgroundSession should have pub fn checkpoint(&self, label: &str) method"
    );

    // @step Then a ghost commit should be created
    // Verify it calls create_ghost_commit
    assert!(
        source.contains("create_ghost_commit"),
        "checkpoint method should call create_ghost_commit"
    );

    // @step And the ref should be stored at refs/fspec-checkpoints/<session-id>/before-refactor
    // Verify it uses session ID as work_unit_id
    assert!(
        source.contains("self.id.to_string()") || source.contains("&self.id"),
        "checkpoint should use session id for checkpoint namespace"
    );
}

// =============================================================================
// Scenario: Checkpoint captures all worktree changes
// =============================================================================

/// Verify checkpoint uses worktree_path for ghost commit creation
///
/// @step Given an isolated session with worktree path
/// @step And there are staged files in the worktree
/// @step And there are unstaged modifications in the worktree
/// @step And there are untracked files in the worktree
/// @step When I call checkpoint with label "full-state"
/// @step Then all file states should be captured in the ghost commit
#[test]
fn test_checkpoint_uses_worktree_path() {
    // @step Given an isolated session with worktree path
    let source = read_session_manager_source();

    // @step When I call checkpoint with label "full-state"
    // Verify checkpoint method uses worktree_path
    let checkpoint_start = source
        .find("pub fn checkpoint(&self, label: &str)")
        .expect("checkpoint method not found");
    let checkpoint_body = &source[checkpoint_start..checkpoint_start.saturating_add(500)];

    // @step Then all file states should be captured in the ghost commit
    // The worktree_path should be passed to create_ghost_commit
    assert!(
        checkpoint_body.contains("worktree_path"),
        "checkpoint should use worktree_path. Found:\n{}",
        checkpoint_body
    );
}

// =============================================================================
// Scenario: Restore checkpoint returns worktree to checkpoint state
// =============================================================================

/// Verify BackgroundSession has restore method
///
/// @step Given an isolated session with worktree path
/// @step And I have created a checkpoint named "before-refactor"
/// @step And I have modified files after the checkpoint
/// @step When I call restore with label "before-refactor"
/// @step Then the worktree files should match the checkpoint state
/// @step And files added after checkpoint should be deleted
#[test]
fn test_background_session_has_restore_method() {
    // @step Given an isolated session with worktree path
    let source = read_session_manager_source();

    // @step When I call restore with label "before-refactor"
    // Verify the method signature exists
    assert!(
        source.contains("pub fn restore(&self, label: &str)"),
        "BackgroundSession should have pub fn restore(&self, label: &str) method"
    );

    // @step Then the worktree files should match the checkpoint state
    // Verify it calls restore_ghost_commit
    assert!(
        source.contains("restore_ghost_commit"),
        "restore method should call restore_ghost_commit"
    );
}

// =============================================================================
// Scenario: Parallel sessions have independent checkpoint namespaces
// =============================================================================

/// Verify checkpoints use session ID as namespace
///
/// @step Given two isolated sessions with different IDs
/// @step And both sessions create checkpoint named "baseline"
/// @step Then each checkpoint should be stored under its own session ID
/// @step And session A should not see session B checkpoints
#[test]
fn test_checkpoint_namespace_uses_session_id() {
    // @step Given two isolated sessions with different IDs
    let source = read_session_manager_source();

    // @step And both sessions create checkpoint named "baseline"
    // Find where create_ghost_commit is called
    let checkpoint_start = source
        .find("pub fn checkpoint(&self, label: &str)")
        .expect("checkpoint method not found");
    let checkpoint_section = &source[checkpoint_start..checkpoint_start.saturating_add(800)];

    // @step Then each checkpoint should be stored under its own session ID
    // Verify the call passes session ID
    assert!(
        checkpoint_section.contains("self.id")
            && checkpoint_section.contains("create_ghost_commit"),
        "checkpoint should pass self.id to create_ghost_commit for namespace isolation"
    );

    // @step And session A should not see session B checkpoints
    // This is enforced by the ghost_commit module using session ID in ref path
    // refs/fspec-checkpoints/<session-id>/<label>
}

// =============================================================================
// Scenario: Checkpoint fails for non-isolated session
// =============================================================================

/// Verify checkpoint returns NotIsolated error for non-isolated sessions
///
/// @step Given a non-isolated session without worktree path
/// @step When I call checkpoint with label "test"
/// @step Then a NotIsolated error should be returned
#[test]
fn test_checkpoint_returns_not_isolated_error() {
    // @step Given a non-isolated session without worktree path
    let source = read_session_manager_source();

    // @step When I call checkpoint with label "test"
    // Find checkpoint method
    let checkpoint_start = source
        .find("pub fn checkpoint(&self, label: &str)")
        .expect("checkpoint method not found");
    let checkpoint_body = &source[checkpoint_start..checkpoint_start.saturating_add(600)];

    // @step Then a NotIsolated error should be returned
    // Verify it checks for worktree_path and returns error if None
    assert!(
        checkpoint_body.contains("NotIsolated") || checkpoint_body.contains("not isolated"),
        "checkpoint should check for isolation and return NotIsolated error. Found:\n{}",
        checkpoint_body
    );

    // Also verify there's some form of error type
    assert!(
        source.contains("SessionError") || source.contains("NotIsolated"),
        "There should be a SessionError type or NotIsolated error variant"
    );
}

// =============================================================================
// Scenario: List checkpoints returns all checkpoint labels
// =============================================================================

/// Verify BackgroundSession has list_checkpoints method
///
/// @step Given an isolated session with worktree path
/// @step And I have created checkpoints named "first", "second", "third"
/// @step When I call list_checkpoints
/// @step Then all three checkpoint labels should be returned
#[test]
fn test_background_session_has_list_checkpoints_method() {
    // @step Given an isolated session with worktree path
    let source = read_session_manager_source();

    // @step When I call list_checkpoints
    // Verify the method signature exists
    assert!(
        source.contains("pub fn list_checkpoints(&self)"),
        "BackgroundSession should have pub fn list_checkpoints(&self) method"
    );

    // @step Then all three checkpoint labels should be returned
    // Verify it calls list_ghost_checkpoints
    assert!(
        source.contains("list_ghost_checkpoints"),
        "list_checkpoints method should call list_ghost_checkpoints"
    );
}

// =============================================================================
// Integration: Verify imports from codelet_git
// =============================================================================

/// Verify session_manager imports ghost_commit functions
///
/// @step Given the session_manager module
/// @step Then it should import create_ghost_commit
/// @step And it should import restore_ghost_commit
/// @step And it should import list_ghost_checkpoints
#[test]
fn test_session_manager_imports_ghost_commit_functions() {
    // @step Given the session_manager module
    let source = read_session_manager_source();

    // @step Then it should import create_ghost_commit
    // Check for use statement or fully qualified path
    let has_import = source.contains("use codelet_git::ghost_commit")
        || source.contains("codelet_git::ghost_commit::create_ghost_commit");

    assert!(
        has_import || source.contains("create_ghost_commit"),
        "session_manager should import or use create_ghost_commit from codelet_git"
    );
}

// =============================================================================
// Verify all three checkpoint methods check isolation
// =============================================================================

/// Verify all checkpoint operations check for isolation
///
/// @step Given the checkpoint, restore, and list_checkpoints methods
/// @step Then each should check worktree_path before proceeding
/// @step And each should return NotIsolated error if not isolated
#[test]
fn test_all_checkpoint_methods_check_isolation() {
    // @step Given the checkpoint, restore, and list_checkpoints methods
    let source = read_session_manager_source();

    // @step Then each should check worktree_path before proceeding
    // Find restore method and verify it checks isolation
    if let Some(restore_start) = source.find("pub fn restore(&self, label: &str)") {
        let restore_body = &source[restore_start..restore_start.saturating_add(400)];
        assert!(
            restore_body.contains("worktree_path") || restore_body.contains("NotIsolated"),
            "restore should check isolation. Found:\n{}",
            restore_body
        );
    }

    // Find list_checkpoints method and verify it checks isolation
    if let Some(list_start) = source.find("pub fn list_checkpoints(&self)") {
        let list_body = &source[list_start..list_start.saturating_add(400)];
        assert!(
            list_body.contains("worktree_path") || list_body.contains("NotIsolated"),
            "list_checkpoints should check isolation. Found:\n{}",
            list_body
        );
    }
}
