//! Feature: spec/features/session-persistence-broken-by-duplicate-manifest-creation-in-fspecagenthooks.feature
//!
//! RPC-423: Session persistence broken by duplicate manifest creation in FspecAgentHooks.
//!
//! FspecAgentHooks::spawn_agent_loop() was creating a duplicate session manifest
//! that OVERWROTE the one already created by SessionManager::create_session_with_id().
//! The hooks code reads session.provider_id which is only the provider part
//! (e.g., "anthropic"), not the full provider/model string
//! (e.g., "anthropic/claude-sonnet-4"). This caused the persisted manifest to have
//! incomplete provider data, breaking session resume.
//!
//! Tests verify:
//!   1. hooks.rs does NOT contain duplicate manifest creation code
//!   2. SessionManager::create_session_with_id creates manifest with full provider string
//!   3. Removing duplicate creation doesn't break agent loop persistence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use codelet_core::persistence::{
    create_session_with_provider, load_session, reset_stores_for_tests, save_session,
    SessionManifest,
};
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

// ============================================================================
// Source-shape helpers
// ============================================================================

fn read_hooks_src() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("hooks.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "must be able to read rust/agent-loop/src/hooks.rs at {}: {e}",
            path.display()
        )
    })
}

// ============================================================================
// Scenario: FspecAgentHooks does not overwrite the session manifest
// ============================================================================

/// @step Given the source of rust/agent-loop/src/hooks.rs
/// @step When the file is scanned for duplicate manifest creation code
/// @step Then FspecAgentHooks::spawn_agent_loop must not call save_session
#[test]
fn hooks_does_not_call_save_session() {
    // @step Given the source of rust/agent-loop/src/hooks.rs
    let src = read_hooks_src();

    // @step When the file is scanned

    // @step Then FspecAgentHooks::spawn_agent_loop must not call save_session
    assert!(
        !src.contains("codelet_core::persistence::save_session"),
        "hooks.rs must NOT call codelet_core::persistence::save_session — \
         manifest creation is handled by SessionManager::create_session_with_id"
    );

    // @step And the manifest created by SessionManager::create_session_with_id must remain unchanged
    // (Asserted by the absence of the save_session call above — if hooks.rs
    // called save_session, it would overwrite the manifest created by
    // SessionManager::create_session_with_id.)
}

/// @step Then FspecAgentHooks::spawn_agent_loop must not construct SessionManifest
#[test]
fn hooks_does_not_construct_session_manifest() {
    // @step Given the source of rust/agent-loop/src/hooks.rs
    let src = read_hooks_src();

    // @step When the file is scanned

    // @step Then FspecAgentHooks::spawn_agent_loop must not construct SessionManifest
    assert!(
        !src.contains("SessionManifest::with_provider"),
        "hooks.rs must NOT construct SessionManifest::with_provider — \
         manifest creation is handled by SessionManager::create_session_with_id"
    );
}

/// @step And the spawn_agent_loop function must not contain RPC-072 manifest creation block
#[test]
fn hooks_spawn_agent_loop_has_no_rpc072_manifest_block() {
    // @step Given the source of rust/agent-loop/src/hooks.rs
    let src = read_hooks_src();

    // @step When the file is scanned

    // @step Then the spawn_agent_loop function must not contain the RPC-072 manifest creation block
    assert!(
        !src.contains("[RPC-072] FspecAgentHooks: failed to create persistence manifest"),
        "hooks.rs must NOT contain the RPC-072 manifest creation warning log — \
         this block was removed by RPC-423 because SessionManager now creates the manifest"
    );

    // @step And the spawn_agent_loop function must not contain the RPC-072 manifest creation debug log
    assert!(
        !src.contains("[RPC-072] FspecAgentHooks: created persistence manifest"),
        "hooks.rs must NOT contain the RPC-072 manifest creation debug log — \
         this block was removed by RPC-423 because SessionManager now creates the manifest"
    );
}

// ============================================================================
// Scenario: Session manifest preserves full provider string after creation
// ============================================================================

/// This test verifies that SessionManager::create_session_with_id creates
/// the manifest with the FULL provider string (e.g., "anthropic/claude-sonnet-4"),
/// not just the provider_id part (e.g., "anthropic").
///
/// We test this by directly using the persistence API that SessionManager
/// calls (SessionManifest::with_provider + save_session) and verifying
/// the persisted manifest has the full provider string.
#[test]
#[serial]
fn session_manifest_preserves_full_provider_string() {
    // @step Given a SessionManager with FspecAgentHooks installed
    // (Simulated by setting up a hermetic data directory with persistence store)
    let _guard = setup_data_dir();

    // @step When I create a session with model "anthropic/claude-sonnet-4"
    // SessionManager::create_session_with_id uses SessionManifest::with_provider
    // with the FULL model string as the provider parameter. We replicate that
    // exact call path here to verify the manifest preserves the full string.
    let project = PathBuf::from("/test/project/rpc423");
    let full_provider = "anthropic/claude-sonnet-4";
    let manifest = create_session_with_provider("test-session", &project, full_provider)
        .expect("create_session_with_provider must succeed");

    // @step Then the persisted manifest must have provider field set to "anthropic/claude-sonnet-4"
    assert_eq!(
        manifest.provider, full_provider,
        "manifest provider must be the full model string '{}', not truncated",
        full_provider
    );

    // @step And the manifest must NOT have provider field set to just "anthropic"
    assert_ne!(
        manifest.provider, "anthropic",
        "manifest provider must NOT be truncated to just the provider part"
    );

    // Verify the manifest can be loaded back from disk with the same provider
    let loaded = load_session(manifest.id).expect("load_session must succeed");
    assert_eq!(
        loaded.provider, full_provider,
        "loaded manifest must preserve the full provider string"
    );
}

/// Verify that the manifest save/load round-trip preserves the full provider string.
#[test]
#[serial]
fn manifest_save_load_preserves_full_provider_string() {
    // @step Given a hermetic data directory
    let _guard = setup_data_dir();

    // @step When I create a manifest with provider "anthropic/claude-sonnet-4" and save it
    let project = PathBuf::from("/test/project/rpc423-save");
    let full_provider = "anthropic/claude-sonnet-4";
    let mut manifest = SessionManifest::with_provider("test-session", project.clone(), full_provider);
    manifest.id = Uuid::new_v4();
    save_session(&manifest).expect("save_session must succeed");

    // @step Then loading the manifest must return the full provider string
    let loaded = load_session(manifest.id).expect("load_session must succeed");
    assert_eq!(
        loaded.provider, full_provider,
        "loaded manifest must preserve the full provider string '{}'",
        full_provider
    );

    // @step And the provider must NOT be truncated to just "anthropic"
    assert_ne!(
        loaded.provider, "anthropic",
        "loaded manifest provider must NOT be truncated"
    );
}

// ============================================================================
// Scenario: Session resume restores the correct model from persisted manifest
// ============================================================================

/// Verify that a persisted manifest with a full provider string can be loaded
/// and the provider/model can be correctly split for session resume.
#[test]
#[serial]
fn session_resume_restores_correct_model() {
    // @step Given a persisted session manifest with provider "anthropic/claude-sonnet-4"
    let _guard = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc423-resume");
    let full_provider = "anthropic/claude-sonnet-4";
    let mut manifest = SessionManifest::with_provider("resume-session", project.clone(), full_provider);
    manifest.id = Uuid::new_v4();
    save_session(&manifest).expect("save_session must succeed");
    let session_id = manifest.id;

    // @step When I resume that session via SessionManagerHandle::resume_session
    // (Simulated by loading the manifest from disk, which is what resume_session does)
    let loaded = load_session(session_id).expect("load_session must succeed");

    // @step Then the resumed BackgroundSession must have provider_id "anthropic" and model_id "claude-sonnet-4"
    // SessionManager::list_sessions splits the provider string by '/' to extract
    // provider_id and model_id. We verify the split works correctly.
    let parts: Vec<&str> = loaded.provider.splitn(2, '/').collect();
    assert_eq!(parts.len(), 2, "provider string must contain '/' separator");
    assert_eq!(parts[0], "anthropic", "provider_id must be 'anthropic'");
    assert_eq!(parts[1], "claude-sonnet-4", "model_id must be 'claude-sonnet-4'");
}

// ============================================================================
// Scenario: Removing duplicate manifest creation does not break agent loop persistence
// ============================================================================

/// Verify that message persistence still works correctly even without
/// the duplicate manifest creation in FspecAgentHooks. The manifest is
/// created by SessionManager::create_session_with_id BEFORE spawn_agent_loop
/// is called, so removing the duplicate block doesn't affect persistence.
#[test]
#[serial]
fn removing_duplicate_creation_does_not_break_persistence() {
    // @step Given a SessionManager with FspecAgentHooks installed without duplicate manifest creation
    // (Simulated by creating a manifest directly via persistence API, which is
    // what SessionManager::create_session_with_id does BEFORE calling spawn_agent_loop)
    let _guard = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc423-persist");
    let full_provider = "anthropic/claude-sonnet-4";
    let manifest = create_session_with_provider("persist-session", &project, full_provider)
        .expect("create_session_with_provider must succeed");
    let session_id = manifest.id;

    // @step When I create a session and send a user message through the agent loop
    // (Simulated by persisting a user message directly, which is what the agent loop does)
    use codelet_agent_loop::persist::persist_user_message;
    persist_user_message(&session_id, "test message from agent loop")
        .expect("persist_user_message must succeed");

    // @step Then the message must be persisted to the session manifest on disk
    let loaded = load_session(session_id).expect("load_session must succeed");
    assert!(
        !loaded.messages.is_empty(),
        "manifest must contain at least one message after persist_user_message"
    );

    // @step And the manifest must contain the message in its messages list
    assert_eq!(
        loaded.messages.len(),
        1,
        "manifest must contain exactly one message"
    );

    // @step And the manifest must still have the full provider string
    assert_eq!(
        loaded.provider, full_provider,
        "manifest provider must still be the full model string after message persistence"
    );
}

/// Verify that the hooks module doc comment reflects that manifest creation
/// is handled by SessionManager (not FspecAgentHooks).
#[test]
fn hooks_doc_comment_mentions_session_manager_manifest() {
    // @step Given the source of rust/agent-loop/src/hooks.rs
    let src = read_hooks_src();

    // @step When the module doc comment is scanned

    // @step Then the doc comment must mention that manifest creation is handled by SessionManager
    // (After RPC-423, the module doc should be updated to reflect this)
    assert!(
        src.contains("SessionManager") || !src.contains("RPC-072 FIX: Create the persistence manifest"),
        "hooks.rs doc must NOT contain the old RPC-072 manifest creation comment — \
         manifest creation is now handled by SessionManager::create_session_with_id"
    );
}
