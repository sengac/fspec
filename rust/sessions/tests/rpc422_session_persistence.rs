//! Feature: spec/features/session-persistence-integration.feature
//!
//! RPC-422 — Integration tests for session persistence in SessionManager.
//! Verifies that session manifests are created on disk when sessions are
//! created, including the provider field, that creation fails when
//! persistence fails, and that creation works offline. The destroy,
//! delete, list and resume scenarios live in
//! rpc422_session_lifecycle_persistence.rs (REFAC-059).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use uuid::Uuid;

mod rpc422_session_persistence_support;
use rpc422_session_persistence_support::{make_temp_data_dir, set_temp_data_dir, DATA_DIR_GUARD};

// ============================================================================
// Scenario: Session creation persists manifest to disk before creating
// BackgroundSession
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_persists_manifest_to_disk() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance with no existing sessions
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step When I call create_session_with_id with a valid UUID, model, project, and name
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    // @step Then a session manifest file should exist at {data_dir}/sessions/{uuid}.json
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(
        manifest_path.exists(),
        "manifest file should exist at {manifest_path:?}"
    );

    // @step And the manifest should contain the session name, project path, and provider
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).expect("valid JSON");
    assert!(
        manifest.get("name").is_some(),
        "manifest should have 'name' field"
    );
    assert!(
        manifest.get("project").is_some(),
        "manifest should have 'project' field"
    );
    assert!(
        manifest.get("provider").is_some(),
        "manifest should have 'provider' field"
    );

    // @step And the manifest should have an empty messages list
    let messages = manifest
        .get("messages")
        .expect("manifest should have 'messages'");
    assert!(
        messages.as_array().unwrap().is_empty(),
        "messages should be empty"
    );

    // @step And the in-memory session map should contain the BackgroundSession with the same UUID
    let sessions = manager
        .list_sessions(
            &std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy(),
        )
        .expect("list_sessions reads the session");
    assert!(
        sessions.iter().any(|s| s.id == sid.value),
        "session should be in memory"
    );
}

// ============================================================================
// Scenario: Session creation persists manifest with provider information
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_with_provider_persists_provider_field() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance with no existing sessions
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step When I call create_session_with_id with model "anthropic/claude-sonnet-4"
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    // @step Then the persisted manifest should have provider field set to "anthropic/claude-sonnet-4"
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).expect("valid JSON");
    let provider = manifest
        .get("provider")
        .expect("provider field")
        .as_str()
        .unwrap();
    assert_eq!(
        provider, "anthropic/claude-sonnet-4",
        "provider should be 'anthropic/claude-sonnet-4', got '{provider}'"
    );
}

// ============================================================================
// Scenario: Session creation persists manifest with provider information
// (duplicate verification with a different model string)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_persists_provider_in_manifest() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step When I call create_session_with_id with a specific model
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    // @step Then the manifest should have the correct provider
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).expect("valid JSON");
    let provider = manifest
        .get("provider")
        .expect("provider field")
        .as_str()
        .unwrap();
    assert_eq!(
        provider, "anthropic/claude-opus-4-5",
        "provider should match the model string"
    );
}

// ============================================================================
// Scenario: Session creation fails gracefully when persistence fails
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_fails_gracefully_when_persistence_fails() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance with a corrupted data directory
    // Use a non-existent data dir path that cannot be written to
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    // Remove the sessions directory so persistence fails
    let sessions_dir = data_dir.join("sessions");
    if sessions_dir.exists() {
        std::fs::remove_dir_all(&sessions_dir).expect("remove the sessions directory");
    }
    // Create a file where the sessions directory should be, preventing directory creation
    std::fs::write(&sessions_dir, b"blocked").expect("block the sessions directory");

    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step When I call create_session_with_id
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let outcome = handle.create_session_checked(None);

    // @step Then the error should propagate and the BackgroundSession should not be created
    // BUG-399: the failure is an Err, never an empty id.
    assert!(
        outcome.is_err(),
        "create_session_checked must fail when persistence fails, got {outcome:?}"
    );
    assert!(
        manager
            .list_live_sessions()
            .expect("list_live_sessions reads the session")
            .is_empty(),
        "no session should be created when persistence fails"
    );

    // Cleanup: remove the blocking file
    std::fs::remove_file(&sessions_dir).expect("remove the blocking file");
}

// ============================================================================
// Scenario: Session persistence tests survive a clean offline environment
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_creation_survives_clean_offline_environment() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a fresh temp data directory without provider credentials or a models.dev cache
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;

    // @step When I run the rpc002 and rpc422 session-persistence test suites in that environment
    // the default model resolves against the seeded offline cache
    assert_eq!(
        manager.set_default_model("anthropic/claude-sonnet-4"),
        Ok(())
    );
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");

    // @step Then session creation resolves the default model from the seeded offline cache and succeeds
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed offline against the seeded cache, got empty SessionId"
    );
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    assert!(
        data_dir
            .join("sessions")
            .join(format!("{uuid}.json"))
            .exists(),
        "manifest must exist after offline session creation"
    );
}
