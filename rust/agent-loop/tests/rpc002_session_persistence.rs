//! Feature: spec/features/session-persistence-round-trip.feature
//!
//! RPC-002: full round-trip tests proving session content written through
//! the agent_loop persist_* helpers lands on disk and survives a store
//! reset plus resume_session: messages, token state and message order.
//! These tests drive the REAL persistence layer (not hand-crafted
//! envelopes). Harness: `rpc002_session_persistence_support` (REFAC-061).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_agent_loop::persist::{
    persist_assistant_message_internal, persist_token_state, persist_user_message,
};
use codelet_core::persistence::{load_session, reset_stores_for_tests, AssistantContent};
use codelet_core::SessionManagerHandle;
use codelet_sessions::SessionManager;
use serial_test::serial;
use uuid::Uuid;

mod rpc002_session_persistence_support;
use rpc002_session_persistence_support::{setup_data_dir, DATA_DIR_GUARD, OFFLINE_DEFAULT_MODEL};

// ============================================================================
// Scenario: Full round-trip — write messages, reset stores, resume, verify
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn full_round_trip_persist_and_restore_messages() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with a session that has persisted messages
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(manager.set_default_model(OFFLINE_DEFAULT_MODEL), Ok(()));
    let handle: &dyn SessionManagerHandle = &*manager;

    // Create a session
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
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
    assert_eq!(
        manifest.messages.len(),
        4,
        "manifest should have 4 message refs"
    );

    // @step When I reset the stores (simulating process restart)
    reset_stores_for_tests();

    // @step And I resume the session
    let result = handle.resume_session(&sid);
    assert!(
        result.is_ok(),
        "resume_session should succeed: {:?}",
        result
    );

    // @step Then the session is in memory with restored messages
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.iter().any(|s| s.id == sid.value),
        "session should be in memory after resume"
    );

    // @step And the manifest still has all 4 message references
    let manifest = load_session(session_uuid).expect("load manifest after resume");
    assert_eq!(
        manifest.messages.len(),
        4,
        "manifest should still have 4 message refs"
    );
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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(manager.set_default_model(OFFLINE_DEFAULT_MODEL), Ok(()));
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    let session_uuid = Uuid::parse_str(&sid.value).expect("valid UUID");

    // Persist a message and token state
    persist_user_message(&session_uuid, "hello").expect("persist user");
    persist_token_state(&session_uuid, 100, 50, 0).expect("persist tokens");

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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(manager.set_default_model(OFFLINE_DEFAULT_MODEL), Ok(()));
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
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
    assert_eq!(
        manifest.messages.len(),
        5,
        "manifest should have 5 message refs"
    );

    // @step When I reset the stores and resume
    reset_stores_for_tests();
    let result = handle.resume_session(&sid);
    assert!(result.is_ok(), "resume_session should succeed");

    // @step Then the manifest still has all 5 message references in order
    let manifest = load_session(session_uuid).expect("load manifest after resume");
    assert_eq!(
        manifest.messages.len(),
        5,
        "manifest should still have 5 message refs"
    );
}
