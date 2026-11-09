//! Feature: spec/features/session-resume-isolation.feature
//!
//! RPC-002: resume_session fails for a session that was never persisted,
//! and two sessions persisted through the agent_loop helpers are restored
//! independently after a store reset. Split from
//! rpc002_session_persistence.rs (REFAC-061).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_agent_loop::persist::persist_user_message;
use codelet_core::persistence::{load_session, reset_stores_for_tests};
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use serial_test::serial;
use uuid::Uuid;

mod rpc002_session_persistence_support;
use rpc002_session_persistence_support::{setup_data_dir, DATA_DIR_GUARD, OFFLINE_DEFAULT_MODEL};

// ============================================================================
// Scenario: Resume fails gracefully for non-existent session
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn resume_fails_for_non_existent_session() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let _data_dir = setup_data_dir();

    // @step Given a SessionManager with no sessions
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(manager.set_default_model(OFFLINE_DEFAULT_MODEL), Ok(()));
    let handle: &dyn SessionManagerHandle = &*manager;

    // @step When I try to resume a session that doesn't exist
    let fake_sid = SessionId::new("00000000-0000-0000-0000-000000000000".to_string());
    let result = handle.resume_session(&fake_sid);

    // @step Then it returns an error
    assert!(
        result.is_err(),
        "resume_session should fail for non-existent session"
    );
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
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(manager.set_default_model(OFFLINE_DEFAULT_MODEL), Ok(()));
    let handle: &dyn SessionManagerHandle = &*manager;

    let sid1 = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    let sid2 = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    let uuid1 = Uuid::parse_str(&sid1.value).expect("valid UUID");
    let uuid2 = Uuid::parse_str(&sid2.value).expect("valid UUID");

    // Persist different messages in each session
    persist_user_message(&uuid1, "session 1 message").expect("persist s1");
    persist_user_message(&uuid2, "session 2 message").expect("persist s2");

    // Verify each manifest has its own messages
    let manifest1 = load_session(uuid1).expect("load manifest 1");
    let manifest2 = load_session(uuid2).expect("load manifest 2");
    assert_eq!(
        manifest1.messages.len(),
        1,
        "session 1 should have 1 message"
    );
    assert_eq!(
        manifest2.messages.len(),
        1,
        "session 2 should have 1 message"
    );

    // @step When I reset the stores and resume both sessions
    reset_stores_for_tests();
    assert!(
        handle.resume_session(&sid1).is_ok(),
        "resume s1 should succeed"
    );
    assert!(
        handle.resume_session(&sid2).is_ok(),
        "resume s2 should succeed"
    );

    // @step Then both sessions are restored independently
    let manifest1 = load_session(uuid1).expect("load manifest 1 after resume");
    let manifest2 = load_session(uuid2).expect("load manifest 2 after resume");
    assert_eq!(
        manifest1.messages.len(),
        1,
        "session 1 should still have 1 message"
    );
    assert_eq!(
        manifest2.messages.len(),
        1,
        "session 2 should still have 1 message"
    );

    // Verify sessions are independent (no cross-contamination)
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.iter().any(|s| s.id == sid1.value),
        "session 1 should be in memory"
    );
    assert!(
        sessions.iter().any(|s| s.id == sid2.value),
        "session 2 should be in memory"
    );
}
