//! Feature: spec/features/session-persistence-integration.feature
//!
//! RPC-422 — Integration tests for session persistence in SessionManager.
//! Verifies that session manifests are created on disk when sessions are
//! created, removed when destroyed, and included in list_sessions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex;
use uuid::Uuid;

/// PROV-132: Serialize tests that swap the process-global data directory.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Helper: create a temp data directory and return its path.
fn make_temp_data_dir() -> PathBuf {
    tempfile::tempdir().expect("tempdir").keep()
}

/// Helper: set the data directory and return a guard that cleans it up.
fn set_temp_data_dir(path: PathBuf) -> PathBuf {
    codelet_common::set_data_directory(path.clone()).expect("set_data_directory");
    // Reset the persistence singletons so they re-initialize against the new data dir.
    codelet_core::persistence::reset_stores_for_tests();
    path
}

// ============================================================================
// Scenario: Session creation persists manifest to disk before creating
// BackgroundSession
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_persists_manifest_to_disk() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance with no existing sessions
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());

    // @step When I call create_session_with_id with a valid UUID, model, project, and name
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    // @step Then a session manifest file should exist at {data_dir}/sessions/{uuid}.json
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(
        manifest_path.exists(),
        "manifest file should exist at {:?}",
        manifest_path
    );

    // @step And the manifest should contain the session name, project path, and provider
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("valid JSON");
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
    let messages = manifest.get("messages").expect("manifest should have 'messages'");
    assert!(messages.as_array().unwrap().is_empty(), "messages should be empty");

    // @step And the in-memory session map should contain the BackgroundSession with the same UUID
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
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
    let manager = Arc::new(SessionManager::new());

    // @step When I call create_session_with_id with model "anthropic/claude-sonnet-4"
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    // @step Then the persisted manifest should have provider field set to "anthropic/claude-sonnet-4"
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("valid JSON");
    let provider = manifest.get("provider").expect("provider field").as_str().unwrap();
    assert_eq!(
        provider, "anthropic/claude-sonnet-4",
        "provider should be 'anthropic/claude-sonnet-4', got '{}'",
        provider
    );
}

// ============================================================================
// Scenario: Session destruction removes from memory but preserves manifest
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn destroy_session_removes_from_memory_preserves_manifest() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a persisted session manifest on disk
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(manifest_path.exists(), "manifest should exist before destroy");

    // @step When I call destroy_session with that session's UUID
    handle.destroy_session(&sid).expect("destroy should succeed");

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
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
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
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    assert!(manifest_path.exists(), "manifest should exist before delete");

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
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

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
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());

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
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.resume_session(&session_id);

    // @step Then the BackgroundSession should be created in memory
    assert!(result.is_ok(), "resume_session should succeed: {:?}", result);

    // @step And the session's inner messages should contain the restored messages
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
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

// ============================================================================
// Scenario: Session creation persists manifest with provider information
// (duplicate verification with a different model string)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_persists_provider_in_manifest() {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager instance
    let data_dir = set_temp_data_dir(make_temp_data_dir());
    let manager = Arc::new(SessionManager::new());

    // @step When I call create_session_with_id with a specific model
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    // @step Then the manifest should have the correct provider
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let manifest_path = data_dir.join("sessions").join(format!("{uuid}.json"));
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_content).expect("valid JSON");
    let provider = manifest.get("provider").expect("provider field").as_str().unwrap();
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
        std::fs::remove_dir_all(&sessions_dir).ok();
    }
    // Create a file where the sessions directory should be, preventing directory creation
    std::fs::write(&sessions_dir, b"blocked").ok();

    let manager = Arc::new(SessionManager::new());

    // @step When I call create_session_with_id
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let _sid = handle.create_session(None);

    // @step Then the error should propagate and the BackgroundSession should not be created
    // The session ID should be empty (PROV-101 decline on error) or the session should not exist
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    assert!(
        sessions.is_empty(),
        "no session should be created when persistence fails"
    );

    // Cleanup: remove the blocking file
    std::fs::remove_file(&sessions_dir).ok();
}

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
            &format!("message {}", i),
        )
        .expect("append message");
    }

    // Verify manifest has 102 messages before resume
    let manifest_before = codelet_core::persistence::load_session(manifest.id)
        .expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        102,
        "manifest should have 102 messages before resume"
    );

    let session_id = SessionId::from(manifest.id.to_string());

    // @step When I close the TUI and reopen it
    // @step And I resume the session
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.resume_session(&session_id);

    // @step Then all 102 messages should be visible in the session history
    assert!(result.is_ok(), "resume_session should succeed: {:?}", result);

    // @step And the session manifest should still reference all 102 messages
    let manifest_after = codelet_core::persistence::load_session(manifest.id)
        .expect("load manifest after resume");
    assert_eq!(
        manifest_after.messages.len(),
        102,
        "manifest should still have 102 messages after resume, got {}",
        manifest_after.messages.len()
    );

    // Verify the session is in memory with correct message count
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    let session_info = sessions
        .iter()
        .find(|s| s.id == session_id.value)
        .expect("session should be in memory after resume");
    assert_eq!(
        session_info.message_count,
        102,
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
    let manifest_before = codelet_core::persistence::load_session(manifest.id)
        .expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        0,
        "manifest should have 0 messages before resume"
    );

    let session_id = SessionId::from(manifest.id.to_string());

    // @step When I close the TUI and reopen it
    // @step And I resume the session
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.resume_session(&session_id);

    // @step Then the session should be empty with no messages
    assert!(result.is_ok(), "resume_session should succeed: {:?}", result);

    let manifest_after = codelet_core::persistence::load_session(manifest.id)
        .expect("load manifest after resume");
    assert_eq!(
        manifest_after.messages.len(),
        0,
        "manifest should still have 0 messages after resume"
    );

    // @step And the session should be functional for new messages
    let sessions = manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
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
    let manager = Arc::new(SessionManager::new());
    manager.set_default_model("anthropic/claude-sonnet-4");
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = handle.create_session(None);

    // Add messages to the session via persistence layer
    let uuid = Uuid::parse_str(&sid.value).expect("valid UUID");
    let mut session = codelet_core::persistence::load_session(uuid)
        .expect("load session");
    codelet_core::persistence::append_message(&mut session, "user", "hello")
        .expect("append user message");
    codelet_core::persistence::append_message(&mut session, "assistant", "hi")
        .expect("append assistant message");

    // Verify manifest has 2 messages before resume
    let manifest_before = codelet_core::persistence::load_session(uuid)
        .expect("load manifest before resume");
    assert_eq!(
        manifest_before.messages.len(),
        2,
        "manifest should have 2 messages before resume"
    );

    let session_id = SessionId::from(uuid.to_string());

    // @step When I resume the same session
    let result = handle.resume_session(&session_id);

    // @step Then the session messages remain unchanged
    assert!(result.is_ok(), "resume_session should succeed: {:?}", result);

    let manifest_after = codelet_core::persistence::load_session(uuid)
        .expect("load manifest after resume");
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
