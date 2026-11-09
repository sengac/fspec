//! Feature: spec/features/resume-session-overwrites-manifest-destroying-message-references.feature
//!
//! SESS-002 — resume_session must not overwrite a persisted manifest's
//! message references, whether the session has many messages, none, or is
//! already in memory. Split from rpc422_session_persistence.rs (REFAC-059).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use uuid::Uuid;

mod rpc422_session_persistence_support;
use rpc422_session_persistence_support::{make_temp_data_dir, set_temp_data_dir, DATA_DIR_GUARD};

// ============================================================================
// Scenario: Resume session preserves manifest message references
// (SESS-002: The bug — resume_session was overwriting the manifest with 0 messages)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_session_preserves_manifest_message_references() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have a session with 102 messages persisted on disk
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    // Create a session via persistence layer
    let manifest = codelet_core::persistence::create_session_with_provider(
        "Resume Test Session",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Add 102 messages to the session
    let mut session = manifest.clone();
    for i in 0..102 {
        codelet_core::persistence::append_message(
            &mut session,
            if i % 2 == 0 { "user" } else { "assistant" },
            &format!("message {i}"),
        )
        .expect("append message");
    }

    // Verify manifest has 102 messages before resume
    let manifest_before =
        codelet_core::persistence::load_session(manifest.id).expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        102,
        "manifest should have 102 messages before resume"
    );

    let session_id = SessionId::from(manifest.id.to_string());

    // @step When I close the TUI and reopen it
    // @step And I resume the session
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

    // @step Then all 102 messages should be visible in the session history
    assert!(result.is_ok(), "resume_session should succeed: {result:?}");

    // @step And the session manifest should still reference all 102 messages
    let manifest_after =
        codelet_core::persistence::load_session(manifest.id).expect("load manifest after resume");
    assert_eq!(
        manifest_after.messages.len(),
        102,
        "manifest should still have 102 messages after resume, got {}",
        manifest_after.messages.len()
    );

    // Verify the session is in memory with correct message count
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    let session_info = sessions
        .iter()
        .find(|s| s.id == session_id.value)
        .expect("session should be in memory after resume");
    assert_eq!(
        session_info.message_count, 102,
        "session in memory should have 102 messages"
    );
}

// ============================================================================
// Scenario: Resume empty session after TUI restart
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_empty_session_preserves_empty_manifest() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have a session with zero messages persisted on disk
    let _data_dir = set_temp_data_dir(make_temp_data_dir());
    let project_path = std::env::current_dir().expect("current dir");

    let manifest = codelet_core::persistence::create_session_with_provider(
        "Empty Session",
        &project_path,
        "anthropic/claude-sonnet-4",
    )
    .expect("create session");

    // Verify manifest has 0 messages before resume
    let manifest_before =
        codelet_core::persistence::load_session(manifest.id).expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        0,
        "manifest should have 0 messages before resume"
    );

    let session_id = SessionId::from(manifest.id.to_string());

    // @step When I close the TUI and reopen it
    // @step And I resume the session
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

    // @step Then the session should be empty with no messages
    assert!(result.is_ok(), "resume_session should succeed: {result:?}");

    let manifest_after =
        codelet_core::persistence::load_session(manifest.id).expect("load manifest after resume");
    assert_eq!(
        manifest_after.messages.len(),
        0,
        "manifest should still have 0 messages after resume"
    );

    // @step And the session should be functional for new messages
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
}

// ============================================================================
// Scenario: Resume session that is already in memory
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_session_already_in_memory_preserves_messages() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given I have a session with messages that is currently active in memory
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

    // Add messages to the session via persistence layer
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let mut session = codelet_core::persistence::load_session(uuid).expect("load session");
    codelet_core::persistence::append_message(&mut session, "user", "hello")
        .expect("append user message");
    codelet_core::persistence::append_message(&mut session, "assistant", "hi")
        .expect("append assistant message");

    // Verify manifest has 2 messages before resume
    let manifest_before =
        codelet_core::persistence::load_session(uuid).expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        2,
        "manifest should have 2 messages before resume"
    );

    let session_id = SessionId::from(uuid.to_string());

    // @step When I resume the same session
    let result = handle.resume_session(&session_id);

    // @step Then the session messages remain unchanged
    assert!(result.is_ok(), "resume_session should succeed: {result:?}");

    let manifest_after =
        codelet_core::persistence::load_session(uuid).expect("load manifest after resume");
    assert_eq!(
        manifest_after.messages.len(),
        2,
        "manifest should still have 2 messages after resume"
    );

    // @step And no manifest overwrite occurs
    // The manifest should not have been rewritten with 0 messages
    assert_eq!(
        manifest_after.messages.len(),
        manifest_before.messages.len(),
        "manifest message count should not change"
    );
}
