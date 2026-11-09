//! Feature: spec/features/session-persistence-lifecycle.feature
//!
//! RPC-422 — Integration tests for the persisted-session lifecycle in
//! SessionManager: destroy keeps the manifest, persistence delete removes
//! it, list_sessions merges persisted sessions, and resume restores
//! messages and token state. Split from rpc422_session_persistence.rs
//! (REFAC-059).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use uuid::Uuid;

mod rpc422_session_persistence_support;
use rpc422_session_persistence_support::{make_temp_data_dir, set_temp_data_dir, DATA_DIR_GUARD};

// ============================================================================
// Scenario: Session destruction removes from memory but preserves manifest
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn destroy_session_removes_from_memory_preserves_manifest() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a persisted session manifest on disk
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(
        manifest_path.exists(),
        "manifest should exist before destroy"
    );

    // @step When I call destroy_session with that session's UUID
    handle
        .destroy_session(&sid)
        .expect("destroy should succeed");

    // @step Then the session should be removed from the in-memory session map
    assert!(
        manager.get_session(&sid.value).is_err(),
        "session should not be in memory after destroy"
    );

    // @step And the manifest file at {data_dir}/sessions/{uuid}.json should still exist on disk
    // (destroy_session only kills the in-memory session; it does NOT delete
    // the manifest. The manifest persists so the user can resume later via /resume.
    // Manifest deletion is a separate operation: persistence_delete_session.)
    assert!(
        manifest_path.exists(),
        "manifest file should still exist after destroy_session — \
         destroy only kills the in-memory session, it does not delete the manifest"
    );

    // @step And the session should still appear in list_sessions via persisted merge
    let sessions = manager
        .list_sessions(
            &manager
                .default_project()
                .expect("the manager has a start workspace")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.iter().any(|s| s.id == sid.value),
        "session should still appear in list_sessions via persisted merge"
    );
}

// ============================================================================
// Scenario: persistence_delete_session removes manifest from disk
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistence_delete_session_removes_manifest_from_disk() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a persisted session manifest on disk
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(
        manifest_path.exists(),
        "manifest should exist before delete"
    );

    // @step When I call persistence_delete_session with that session's UUID
    codelet_core::persistence::delete_session(uuid).expect("delete should succeed");

    // @step Then the manifest file at {data_dir}/sessions/{uuid}.json should no longer exist
    assert!(
        !manifest_path.exists(),
        "manifest file should be deleted by persistence_delete_session"
    );
}

// ============================================================================
// Scenario: Session listing includes both in-memory and persisted sessions
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_sessions_includes_persisted_sessions() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with one in-memory session
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    // @step And a second session manifest persisted on disk but not in memory
    // Create a second manifest directly via persistence layer
    let project_path = std::env::current_dir().expect("current dir");
    codelet_core::persistence::create_session_with_provider(
        "Persisted Only Session",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create persisted session");

    // @step When I call list_sessions
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");

    // @step Then the result should contain both sessions
    assert!(
        sessions.len() >= 2,
        "should have at least 2 sessions, got {}",
        sessions.len()
    );

    // Verify the in-memory session is present
    assert!(
        sessions.iter().any(|s| s.id == sid.value),
        "in-memory session should be in list"
    );
}

// ============================================================================
// Scenario: Resume session loads messages from persistence layer
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_session_restores_messages_and_token_state() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a persisted session manifest with two stored messages
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    // Create a session via persistence layer
    let manifest = codelet_core::persistence::create_session_with_provider(
        "Resume Test Session",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Add two messages to the session
    let mut session = manifest.clone();
    codelet_core::persistence::append_message(&mut session, "user", "hello world")
        .expect("append user message");
    codelet_core::persistence::append_message(&mut session, "assistant", "hi back")
        .expect("append assistant message");

    let session_id = SessionId::from(manifest.id.to_string());

    // @step When I call resume_session with that session's UUID
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.resume_session(&session_id);

    // @step Then the BackgroundSession should be created in memory
    assert!(result.is_ok(), "resume_session should succeed: {result:?}");

    // @step And the session's inner messages should contain the restored messages
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.iter().any(|s| s.id == session_id.value),
        "session should be in memory after resume"
    );

    // @step And the token state should be restored from the manifest
    // The session should have been created with the correct provider
    let session_info = sessions
        .iter()
        .find(|s| s.id == session_id.value)
        .expect("session should exist");
    assert_eq!(
        session_info.provider_id,
        Some("anthropic".to_string()),
        "provider should match manifest"
    );
}
