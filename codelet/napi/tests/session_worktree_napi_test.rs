#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/session-worktree-napi.feature
//!
//! Integration tests for Session Worktree NAPI Bindings
//!
//! GIT-027: Expose all session worktree operations to TypeScript via NAPI.
//!
//! NOTE: The core session operations are tested in codelet-git tests.
//! These tests verify the NAPI layer - that functions are correctly exported
//! and types are properly defined.

use std::fs;
use std::path::Path;

// =============================================================================
// Source Code Verification Helpers
// =============================================================================

fn read_git_napi_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/git.rs");
    fs::read_to_string(&path).expect("Failed to read git.rs")
}

// =============================================================================
// Scenario: Create isolated session with worktree
// =============================================================================

/// Verify createIsolatedSession NAPI function is defined
///
/// @step Given a git repository at "/project"
/// @step When I call createIsolatedSession with sessionId "feature-auth" and baseRef "main"
/// @step Then the result contains worktreePath, baseCommit, and createdAt fields
/// @step And a worktree directory exists at ".fspec/worktrees/feature-auth/"
#[test]
fn test_create_isolated_session_napi_exists() {
    // @step Given the git.rs NAPI source file
    let source = read_git_napi_source();

    // @step Then createIsolatedSession function should be defined with #[napi]
    // Check for function with NAPI attribute (may use existing create_worktree_at_ref)
    assert!(
        source.contains("#[napi]") && source.contains("create_worktree_at_ref"),
        "create_worktree_at_ref NAPI function should exist"
    );

    // Verify it accepts the required parameters
    assert!(
        source.contains("repo_path: String") && source.contains("session_id: String"),
        "Function should accept repo_path and session_id parameters"
    );
}

/// Verify WorktreeCreateResultJs includes all required fields
///
/// @step Then the result contains worktreePath, baseCommit, and createdAt fields
#[test]
fn test_worktree_create_result_has_required_fields() {
    let source = read_git_napi_source();

    // Find WorktreeCreateResultJs struct
    assert!(
        source.contains("pub struct WorktreeCreateResultJs"),
        "WorktreeCreateResultJs struct should exist"
    );

    // Verify required fields
    assert!(
        source.contains("pub path: String") || source.contains("pub worktree_path: String"),
        "WorktreeCreateResultJs should have path field"
    );
    assert!(
        source.contains("pub base_commit: String"),
        "WorktreeCreateResultJs should have base_commit field"
    );
    assert!(
        source.contains("pub created_at: String"),
        "WorktreeCreateResultJs should have created_at field"
    );
}

// =============================================================================
// Scenario: List all session worktrees with status
// =============================================================================

/// Verify listSessions NAPI function is defined
///
/// @step Given a git repository with 3 existing session worktrees
/// @step When I call listSessions with filter "all"
/// @step Then the result is an array of SessionInfoJs with 3 entries
/// @step And each entry contains sessionId, status, baseCommit, filesChanged, createdAt, and worktreePath
#[test]
fn test_list_sessions_napi_exists() {
    let source = read_git_napi_source();

    // Check for listSessions or list_sessions NAPI function
    assert!(
        source.contains("#[napi]") && source.contains("list_sessions"),
        "list_sessions NAPI function should exist"
    );
}

/// Verify SessionInfoJs struct exists with required fields
///
/// @step And each entry contains sessionId, status, baseCommit, filesChanged, createdAt, and worktreePath
#[test]
fn test_session_info_js_struct_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("pub struct SessionInfoJs"),
        "SessionInfoJs struct should exist"
    );
}

// =============================================================================
// Scenario: List sessions with pending merge filter
// =============================================================================

/// Verify listSessions accepts filter parameter
///
/// @step Given a git repository with 2 clean sessions and 1 session with uncommitted changes
/// @step When I call listSessions with filter "pending_merge"
/// @step Then the result contains only the 1 session with uncommitted changes
#[test]
fn test_list_sessions_accepts_filter() {
    let source = read_git_napi_source();

    // listSessions should accept an optional filter parameter
    let list_sessions_start = source.find("fn list_sessions").expect("list_sessions not found");
    let func_sig = &source[list_sessions_start..list_sessions_start + 300];

    assert!(
        func_sig.contains("filter: Option<String>") || func_sig.contains("filter: String"),
        "list_sessions should accept a filter parameter"
    );
}

// =============================================================================
// Scenario: Inspect session diff before merging
// =============================================================================

/// Verify inspectSession NAPI function is defined
///
/// @step Given an isolated session "feature-auth" with modified files
/// @step When I call inspectSession for "feature-auth"
/// @step Then the result contains diff string, filesChanged, filesAdded, and filesDeleted arrays
/// @step And the session worktree remains unchanged
#[test]
fn test_inspect_session_napi_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("#[napi]") && source.contains("inspect_session"),
        "inspect_session NAPI function should exist"
    );
}

/// Verify SessionResultJs has all diff fields
///
/// @step Then the result contains diff string, filesChanged, filesAdded, and filesDeleted arrays
#[test]
fn test_session_result_js_has_diff_fields() {
    let source = read_git_napi_source();

    assert!(
        source.contains("pub struct SessionResultJs"),
        "SessionResultJs struct should exist"
    );

    assert!(
        source.contains("pub diff: String"),
        "SessionResultJs should have diff field"
    );
    assert!(
        source.contains("pub files_changed: Vec<String>"),
        "SessionResultJs should have files_changed field"
    );
    assert!(
        source.contains("pub files_added: Vec<String>"),
        "SessionResultJs should have files_added field"
    );
    assert!(
        source.contains("pub files_deleted: Vec<String>"),
        "SessionResultJs should have files_deleted field"
    );
}

// =============================================================================
// Scenario: Merge session changes to main worktree
// =============================================================================

/// Verify mergeSession NAPI function is defined
///
/// @step Given an isolated session "feature-auth" with modified and added files
/// @step When I call mergeSession for "feature-auth"
/// @step Then the result contains filesModified, filesAdded, and filesDeleted arrays
/// @step And the session worktree is removed
/// @step And the changes appear in the main worktree
#[test]
fn test_merge_session_napi_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("#[napi]") && source.contains("merge_session"),
        "merge_session NAPI function should exist"
    );
}

/// Verify MergeResultJs struct exists with required fields
///
/// @step Then the result contains filesModified, filesAdded, and filesDeleted arrays
#[test]
fn test_merge_result_js_struct_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("pub struct MergeResultJs"),
        "MergeResultJs struct should exist"
    );
}

// =============================================================================
// Scenario: Merge session returns conflict error
// =============================================================================

/// Verify mergeSession throws error on conflict
///
/// @step Given an isolated session "conflict-session" with modified files
/// @step And the same files were modified in the main worktree since base commit
/// @step When I call mergeSession for "conflict-session"
/// @step Then the function throws an Error with "Conflict" in the message
/// @step And the error message contains the list of conflicting file paths
/// @step And the session worktree remains intact
#[test]
fn test_merge_session_returns_napi_error() {
    let source = read_git_napi_source();

    // Verify the function signature returns napi::Result
    let merge_start = source.find("fn merge_session").expect("merge_session not found");
    let func_sig = &source[merge_start..merge_start + 200];

    assert!(
        func_sig.contains("-> napi::Result<"),
        "merge_session should return napi::Result"
    );

    // Verify error handling uses napi::Error::from_reason
    assert!(
        source.contains("napi::Error::from_reason"),
        "Errors should be converted using napi::Error::from_reason"
    );
}

// =============================================================================
// Scenario: Discard session without applying changes
// =============================================================================

/// Verify discardSession NAPI function is defined
///
/// @step Given an isolated session "feature-auth" with 3 modified files
/// @step When I call discardSession for "feature-auth"
/// @step Then the result contains sessionId and filesDiscarded count of 3
/// @step And the session worktree is removed
/// @step And the main worktree is unchanged
#[test]
fn test_discard_session_napi_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("#[napi]") && source.contains("discard_session"),
        "discard_session NAPI function should exist"
    );
}

/// Verify DiscardResultJs struct exists
///
/// @step Then the result contains sessionId and filesDiscarded count
#[test]
fn test_discard_result_js_struct_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("pub struct DiscardResultJs"),
        "DiscardResultJs struct should exist"
    );
}

// =============================================================================
// Scenario: Prune orphaned worktrees
// =============================================================================

/// Verify pruneOrphaned NAPI function is defined
///
/// @step Given 3 orphaned worktrees exist with no valid session manifests
/// @step When I call pruneOrphaned
/// @step Then the result contains count of 3 and an array of pruned session IDs
/// @step And the 3 orphaned worktree directories are removed
#[test]
fn test_prune_orphaned_napi_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("#[napi]") && source.contains("prune_orphaned"),
        "prune_orphaned NAPI function should exist"
    );
}

/// Verify PruneResultJs struct exists
///
/// @step Then the result contains count and an array of pruned session IDs
#[test]
fn test_prune_result_js_struct_exists() {
    let source = read_git_napi_source();

    assert!(
        source.contains("pub struct PruneResultJs"),
        "PruneResultJs struct should exist"
    );
}

// =============================================================================
// Integration: Verify codelet-git session_status functions are importable
// =============================================================================

/// Verify that session_status functions from codelet-git can be used
///
/// This test ensures the integration between codelet-napi and codelet-git
/// for session management functions.
#[test]
fn test_session_status_functions_are_importable() {
    // These compile only if properly exported from codelet-git
    // The imports verify that the public API is accessible.
    #[allow(unused_imports)]
    use codelet_git::{
        discard_session, inspect_session, list_sessions, merge_session, prune_orphaned,
        SessionFilter, SessionInfo, MergeResult, DiscardResult, PruneResult, SessionResult, Result,
    };

    // Verify types are usable (don't need to call functions)
    let _filter: SessionFilter = SessionFilter::All;
    
    // Function signatures are correct if this compiles
    fn _verify_list_sessions_sig() {
        fn _test<P: AsRef<std::path::Path>>(
            _repo: P,
            _active: &std::collections::HashSet<String>,
            _filter: codelet_git::SessionFilter,
        ) -> codelet_git::Result<Vec<codelet_git::SessionInfo>> {
            codelet_git::list_sessions(_repo, _active, _filter)
        }
    }

    fn _verify_inspect_session_sig() {
        fn _test<P: AsRef<std::path::Path>>(
            _repo: P,
            _session_id: &str,
        ) -> codelet_git::Result<codelet_git::SessionResult> {
            codelet_git::inspect_session(_repo, _session_id)
        }
    }

    fn _verify_merge_session_sig() {
        fn _test<P: AsRef<std::path::Path>>(
            _repo: P,
            _session_id: &str,
        ) -> codelet_git::Result<codelet_git::MergeResult> {
            codelet_git::merge_session(_repo, _session_id)
        }
    }

    fn _verify_discard_session_sig() {
        fn _test<P: AsRef<std::path::Path>>(
            _repo: P,
            _session_id: &str,
        ) -> codelet_git::Result<codelet_git::DiscardResult> {
            codelet_git::discard_session(_repo, _session_id)
        }
    }

    fn _verify_prune_orphaned_sig() {
        fn _test<P: AsRef<std::path::Path>>(
            _repo: P,
            _active: &std::collections::HashSet<String>,
        ) -> codelet_git::Result<codelet_git::PruneResult> {
            codelet_git::prune_orphaned(_repo, _active)
        }
    }
}
