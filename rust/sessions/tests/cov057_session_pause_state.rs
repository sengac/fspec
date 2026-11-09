//! Feature: spec/features/session-pause-state-isolation.feature
//!
//! COV-057: pause state lives on each `BackgroundSession` (the former NAPI
//! `session_get_pause_state` binding is now `SessionManagerHandle::
//! get_pause_state`). Two real sessions are created through the handle as in
//! git021_checkpoint_requires_isolation.rs; pausing one must leave the other
//! untouched.
//!
//! multi-thread runtime is REQUIRED: `create_session_checked` bridges
//! sync->async via `tokio::task::block_in_place`.

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::SessionManager;
use codelet_tools::tool_pause::{PauseKind, PauseState};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_pause_state_is_stored_per_session_not_globally() -> TestResult {
    let data_dir = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    offline_catalog::seed_offline_catalog()?;
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    let handle: &dyn SessionManagerHandle = &*manager;

    // @step Given two sessions exist
    let id_a = handle.create_session_checked(None)?;
    let id_b = handle.create_session_checked(None)?;
    let session_a = manager.get_session(&id_a.value)?;
    let session_b = manager.get_session(&id_b.value)?;
    // Both sessions are mid-turn when the pause happens.
    session_a.set_status(SessionStatus::Running);
    session_b.set_status(SessionStatus::Running);

    // @step When session A pauses
    session_a.set_pause_state(Some(PauseState {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Run a destructive command?".to_string(),
        details: Some("rm -rf build".to_string()),
        tool_call_id: None,
    }));

    // @step Then session A status should be "paused"
    assert_eq!(handle.get_session_status(&id_a)?, SessionStatus::Paused);
    // @step And session B status should be "running"
    assert_eq!(handle.get_session_status(&id_b)?, SessionStatus::Running);
    // @step And querying session A pause state returns the details
    let paused = session_a
        .get_pause_state()
        .ok_or("session A lost its pause state")?;
    assert!(matches!(paused.kind, PauseKind::Confirm));
    assert_eq!(paused.tool_name, "Bash");
    assert_eq!(paused.message, "Run a destructive command?");
    assert_eq!(paused.details.as_deref(), Some("rm -rf build"));
    assert!(handle.get_pause_state(&id_a)?.is_some());
    // @step And querying session B pause state returns null
    assert!(session_b.get_pause_state().is_none());
    assert!(handle.get_pause_state(&id_b)?.is_none());
    Ok(())
}
