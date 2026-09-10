#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/isolated-session-creation.feature
//!
//! Integration tests for Isolated Session Creation and effective_cwd
//!
//! GIT-019: Add isolated parameter to session creation, track worktree info,
//! implement effective_cwd() method.
//!
//! NOTE: The core isolation logic is tested via codelet-git create_worktree /
//! create_worktree_at_ref (WT-011: IsolatedSessionInfo was deleted).
//! These tests verify the NAPI layer integration - that BackgroundSession
//! correctly stores and exposes the isolation fields. The BackgroundSession
//! struct now lives in the codelet-sessions crate
//! (rust/sessions/src/background_session.rs).

use std::fs;
use std::path::Path;

// =============================================================================
// Source Code Verification Helpers
// =============================================================================

fn read_background_session_source() -> String {
    // BackgroundSession was lifted from codelet-napi into the codelet-sessions
    // crate (3affacdd: major architecture overhaul).
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sessions/src/background_session.rs");
    fs::read_to_string(&path).expect("Failed to read background_session.rs")
}

fn read_session_manager_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sessions/src/session_manager.rs");
    fs::read_to_string(&path).expect("Failed to read session_manager.rs")
}

// =============================================================================
// Scenario: BackgroundSession has worktree_path field
// =============================================================================

/// Verify BackgroundSession struct has worktree_path field
///
/// @step Given the BackgroundSession struct in session_manager.rs
/// @step Then it should have a worktree_path field of type Option<PathBuf>
#[test]
fn test_background_session_has_worktree_path_field() {
    // @step Given the BackgroundSession struct in session_manager.rs
    let source = read_background_session_source();

    // @step Then it should have a worktree_path field of type Option<PathBuf>
    // WT-009: interior-mutable (RwLock) so detach can clear it in place,
    // still an Option<PathBuf>.
    assert!(
        source.contains("pub worktree_path: RwLock<Option<PathBuf>>"),
        "BackgroundSession should have a worktree_path field of type Option<PathBuf>"
    );

    // Verify it has the GIT-019 documentation
    assert!(
        source.contains("GIT-019: Path to worktree for isolated sessions"),
        "worktree_path field should have GIT-019 documentation"
    );
}

// =============================================================================
// Scenario: BackgroundSession has base_commit field
// =============================================================================

/// Verify BackgroundSession struct has base_commit field
///
/// @step Given the BackgroundSession struct in session_manager.rs
/// @step Then it should have a base_commit field of type Option<String>
#[test]
fn test_background_session_has_base_commit_field() {
    let source = read_background_session_source();

    // @step Then it should have a base_commit field of type Option<String>
    assert!(
        source.contains("pub base_commit: RwLock<Option<String>>"),
        "BackgroundSession should have a base_commit field of type Option<String>"
    );

    assert!(
        source.contains("GIT-019: Base commit SHA for isolated sessions"),
        "base_commit field should have GIT-019 documentation"
    );
}

// =============================================================================
// Scenario: BackgroundSession has effective_cwd method
// =============================================================================

/// Verify BackgroundSession has effective_cwd method that returns PathBuf
///
/// @step Given the BackgroundSession impl in session_manager.rs
/// @step Then it should have an effective_cwd method
/// @step And the method should return PathBuf
#[test]
fn test_background_session_has_effective_cwd_method() {
    let source = read_background_session_source();

    // Check for the method signature
    assert!(
        source.contains("pub fn effective_cwd(&self) -> PathBuf"),
        "BackgroundSession should have pub fn effective_cwd(&self) -> PathBuf method"
    );

    // Check for GIT-019 documentation
    assert!(
        source.contains("GIT-019: Returns the effective working directory"),
        "effective_cwd method should have GIT-019 documentation"
    );
}

// =============================================================================
// Scenario: effective_cwd returns worktree_path when Some
// =============================================================================

/// Verify effective_cwd uses worktree_path.unwrap_or pattern
///
/// @step Given the effective_cwd implementation
/// @step Then it should return worktree_path when Some
/// @step And it should return project root when None
#[test]
fn test_effective_cwd_uses_correct_pattern() {
    let source = read_background_session_source();

    // Find the effective_cwd method body
    let method_start = source
        .find("pub fn effective_cwd(&self) -> PathBuf")
        .expect("effective_cwd method not found");
    let method_body = &source[method_start..method_start + 400];

    // Verify it uses the correct pattern
    assert!(
        method_body.contains("worktree_path") && method_body.contains("unwrap_or"),
        "effective_cwd should use worktree_path.unwrap_or pattern. Found: {}",
        method_body
    );
}

// =============================================================================
// Scenario: Default session creation passes None for worktree
// =============================================================================

/// Verify default session creation passes None for worktree_path and base_commit
///
/// @step Given the create_session_with_id implementation
/// @step When a session is created without isolation
/// @step Then worktree_path should be None
/// @step And base_commit should be None
#[test]
fn test_default_session_creation_passes_none_for_worktree() {
    let source = read_session_manager_source();

    // Find BackgroundSession::new call in create_session_with_id
    // It should pass None for both worktree_path and base_commit
    assert!(
        source.contains("worktree_path: None"),
        "Default session creation should pass None for worktree_path"
    );

    assert!(
        source.contains("base_commit: None"),
        "Default session creation should pass None for base_commit"
    );
}

// =============================================================================
// Scenario: BackgroundSession::new accepts worktree parameters
// =============================================================================

/// Verify BackgroundSession::new constructor accepts worktree parameters
///
/// @step Given the BackgroundSession::new signature
/// @step Then it should accept worktree_path: Option<PathBuf>
/// @step And it should accept base_commit: Option<String>
#[test]
fn test_background_session_new_accepts_worktree_params() {
    let source = read_background_session_source();

    // Find the BackgroundSession::new signature (marked by its GIT-019 doc line)
    let doc_start = source
        .find("GIT-019: Added worktree_path and base_commit parameters")
        .expect("BackgroundSession::new GIT-019 doc not found");
    let new_method_sig = &source[doc_start..doc_start + 900];

    assert!(
        new_method_sig.contains("worktree_path: Option<PathBuf>"),
        "BackgroundSession::new should accept worktree_path parameter"
    );

    assert!(
        new_method_sig.contains("base_commit: Option<String>"),
        "BackgroundSession::new should accept base_commit parameter"
    );
}
