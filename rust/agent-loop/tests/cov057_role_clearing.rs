//! Feature: spec/features/role-clearing-via-napi.feature
//!
//! COV-057: the NAPI `session_set_role` binding is gone; the surviving
//! "empty role clears the role" logic is the AgentManager `set_role`
//! action (`agent_manager_handler::handle_set_role`). This drives the
//! real handler against a real, non-singleton `SessionManager`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_agent_loop::agent_manager_handler::create_handler;
use codelet_core::persistence::reset_stores_for_tests;
use codelet_sessions::SessionManager;
use codelet_tools::{AgentManagerAction, AgentManagerResult};
use serial_test::serial;
use uuid::Uuid;

/// Offline models.dev catalog so registry validation never hits the network.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");
const MODEL: &str = "anthropic/claude-opus-4-5";

fn isolated_manager() -> Result<(tempfile::TempDir, Arc<SessionManager>), String> {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = SessionManager::new();
    manager.set_default_model(MODEL)?;
    Ok((data_dir, manager))
}

/// Scenario: session_set_role with empty string clears the role
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_set_role_with_empty_string_clears_the_role() -> Result<(), String> {
    let (_data_dir, manager) = isolated_manager()?;
    let project_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = project_dir.path().to_string_lossy().to_string();

    // @step Given a session exists with role "reviewer"
    let session_id = Uuid::new_v4();
    manager
        .create_session_with_id(&session_id.to_string(), MODEL, &project, "role-target")
        .await?;
    let session = manager.get_session(&session_id.to_string())?;
    session.set_role("reviewer".to_string());
    assert_eq!(session.get_role().as_deref(), Some("reviewer"));

    // @step When the AgentManager set_role action is called with an empty role
    let handler = create_handler(
        Some(manager.clone()),
        project,
        Some(MODEL.to_string()),
        None,
        None,
    );
    let result = handler(
        AgentManagerAction::SetRole {
            session_id: None,
            role: String::new(),
        },
        session_id,
    );

    // @step Then the session role should be cleared
    match result {
        AgentManagerResult::RoleSet {
            session_id: target,
            role,
        } => {
            assert_eq!(target, session_id.to_string());
            assert_eq!(role, None, "RoleSet must report the role as cleared");
        }
        other => panic!("expected RoleSet, got: {other:?}"),
    }

    // @step And get_role on the session should return None
    let reloaded = manager.get_session(&session_id.to_string())?;
    assert_eq!(reloaded.get_role(), None);
    Ok(())
}
