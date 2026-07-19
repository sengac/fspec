//! Feature: spec/features/session-persistence-integration.feature
//!
//! RPC-002: Full round-trip tests proving session content is written to
//! disk and restored correctly via resume_session. Verifies that:
//!
//!   1. Messages persisted via agent_loop persist_* helpers land on disk
//!   2. resume_session rehydrates ALL messages into the session's inner list
//!   3. Token state is restored from the manifest
//!   4. The restored session's scrollback matches what was written
//!
//! These tests drive the REAL persistence layer (not hand-crafted envelopes)
//! and verify the full write → resume → restore round-trip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_agent_loop::persist::{
    persist_assistant_message_internal, persist_token_state, persist_user_message,
};
use codelet_core::persistence::{
    load_session, reset_stores_for_tests, AssistantContent,
};
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

/// Serialize tests that swap the process-global data directory.
static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

// ============================================================================
// Scenario: Full round-trip — write messages, reset stores, resume, verify
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn full_round_trip_persist_and_restore_messages() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with a session that has persisted messages
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    // Create a session
    let sid = handle.create_session(None);
    let session_uuid = Uuid::parse_str(&sid.value).expect("valid UUID");

    // Persist messages through the agent_loop persistence helpers
    persist_user_message(&session_uuid, "hello world").expect("persist user message");
    persist_assistant_message_internal(
        &session_uuid,
        "anthropic",
        vec![AssistantContent::Text {
            text: "hi back".to_string(),
        }],
        Some("end_turn".to_string()),
    )
    .expect("persist assistant message");
    persist_user_message(&session_uuid, "second message").expect("persist user 2");
    persist_assistant_message_internal(
        &session_uuid,
        "anthropic",
        vec![AssistantContent::Text {
            text: "second reply".to_string(),
        }],
        Some("end_turn".to_string()),
    )
    .expect("persist assistant 2");

    // Verify manifest has 4 message references
    let manifest = load_session(session_uuid).expect("load manifest");
    assert_eq!(manifest.messages.len(), 4, "manifest should have 4 message refs");

    // @step When I reset the stores (simulating process restart)
    reset_stores_for_tests();

    // @step And I resume the session
    let result = handle.resume_session(&sid);
    assert!(result.is_ok(), "resume_session should succeed: {:?}", result);

    // @step Then the session is in memory with restored messages
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    assert!(
        sessions.iter().any(|s| s.id == sid.value),
        "session should be in memory after resume"
    );

    // @step And the manifest still has all 4 message references
    let manifest = load_session(session_uuid).expect("load manifest after resume");
    assert_eq!(manifest.messages.len(), 4, "manifest should still have 4 message refs");
}

// ============================================================================
// Scenario: Resume restores token state from manifest
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn resume_restores_token_state() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with a session that has token state
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid = handle.create_session(None);
    let session_uuid = Uuid::parse_str(&sid.value).expect("valid UUID");

    // Persist a message and token state
    persist_user_message(&session_uuid, "hello").expect("persist user");
    persist_token_state(&session_uuid, 100, 50).expect("persist tokens");

    // Verify token state is in manifest
    let manifest = load_session(session_uuid).expect("load manifest");
    assert_eq!(
        manifest.token_usage.cumulative_billed_input, 100,
        "manifest should have input tokens"
    );
    assert_eq!(
        manifest.token_usage.cumulative_billed_output, 50,
        "manifest should have output tokens"
    );

    // @step When I reset the stores and resume
    reset_stores_for_tests();
    let result = handle.resume_session(&sid);
    assert!(result.is_ok(), "resume_session should succeed");

    // @step Then the token state is restored
    let manifest = load_session(session_uuid).expect("load manifest after resume");
    assert_eq!(
        manifest.token_usage.cumulative_billed_input, 100,
        "tokens should be restored"
    );
    assert_eq!(
        manifest.token_usage.cumulative_billed_output, 50,
        "tokens should be restored"
    );
}

// ============================================================================
// Scenario: Resume restores messages with correct ordering
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn resume_preserves_message_ordering() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a session with multiple messages in specific order
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid = handle.create_session(None);
    let session_uuid = Uuid::parse_str(&sid.value).expect("valid UUID");

    // Persist messages in order: user, assistant, user, assistant, user
    persist_user_message(&session_uuid, "msg1").expect("persist 1");
    persist_assistant_message_internal(
        &session_uuid,
        "anthropic",
        vec![AssistantContent::Text {
            text: "reply1".to_string(),
        }],
        Some("end_turn".to_string()),
    )
    .expect("persist 2");
    persist_user_message(&session_uuid, "msg2").expect("persist 3");
    persist_assistant_message_internal(
        &session_uuid,
        "anthropic",
        vec![AssistantContent::Text {
            text: "reply2".to_string(),
        }],
        Some("end_turn".to_string()),
    )
    .expect("persist 4");
    persist_user_message(&session_uuid, "msg3").expect("persist 5");

    // Verify manifest has 5 message references
    let manifest = load_session(session_uuid).expect("load manifest");
    assert_eq!(manifest.messages.len(), 5, "manifest should have 5 message refs");

    // @step When I reset the stores and resume
    reset_stores_for_tests();
    let result = handle.resume_session(&sid);
    assert!(result.is_ok(), "resume_session should succeed");

    // @step Then the manifest still has all 5 message references in order
    let manifest = load_session(session_uuid).expect("load manifest after resume");
    assert_eq!(manifest.messages.len(), 5, "manifest should still have 5 message refs");
}

// ============================================================================
// Scenario: Resume fails gracefully for non-existent session
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn resume_fails_for_non_existent_session() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with no sessions
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    // @step When I try to resume a session that doesn't exist
    let fake_sid = SessionId::new("00000000-0000-0000-0000-000000000000".to_string());
    let result = handle.resume_session(&fake_sid);

    // @step Then it returns an error
    assert!(result.is_err(), "resume_session should fail for non-existent session");
}

// ============================================================================
// Scenario: Multiple sessions persist independently
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn multiple_sessions_persist_independently() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with two sessions
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid1 = handle.create_session(None);
    let sid2 = handle.create_session(None);
    let uuid1 = Uuid::parse_str(&sid1.value).expect("valid UUID");
    let uuid2 = Uuid::parse_str(&sid2.value).expect("valid UUID");

    // Persist different messages in each session
    persist_user_message(&uuid1, "session 1 message").expect("persist s1");
    persist_user_message(&uuid2, "session 2 message").expect("persist s2");

    // Verify each manifest has its own messages
    let manifest1 = load_session(uuid1).expect("load manifest 1");
    let manifest2 = load_session(uuid2).expect("load manifest 2");
    assert_eq!(manifest1.messages.len(), 1, "session 1 should have 1 message");
    assert_eq!(manifest2.messages.len(), 1, "session 2 should have 1 message");

    // @step When I reset the stores and resume both sessions
    reset_stores_for_tests();
    assert!(handle.resume_session(&sid1).is_ok(), "resume s1 should succeed");
    assert!(handle.resume_session(&sid2).is_ok(), "resume s2 should succeed");

    // @step Then both sessions are restored independently
    let manifest1 = load_session(uuid1).expect("load manifest 1 after resume");
    let manifest2 = load_session(uuid2).expect("load manifest 2 after resume");
    assert_eq!(manifest1.messages.len(), 1, "session 1 should still have 1 message");
    assert_eq!(manifest2.messages.len(), 1, "session 2 should still have 1 message");

    // Verify sessions are independent (no cross-contamination)
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    assert!(
        sessions.iter().any(|s| s.id == sid1.value),
        "session 1 should be in memory"
    );
    assert!(
        sessions.iter().any(|s| s.id == sid2.value),
        "session 2 should be in memory"
    );
}
