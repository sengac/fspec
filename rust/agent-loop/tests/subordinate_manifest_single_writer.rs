//! Feature: spec/features/subordinate-manifest-single-writer.feature

use codelet_agent_loop::agent_manager_handler::create_handler;
use codelet_core::persistence::{load_session, reset_stores_for_tests, SessionStore};
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use codelet_tools::agent_manager::{AgentManagerAction, AgentManagerResult};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MODEL: &str = "codex/gpt-5";

async fn setup(
) -> Result<(tempfile::TempDir, Arc<SessionManager>, Uuid), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let cache = directory.path().join("cache");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(
        cache.join("models.json"),
        include_str!("fixtures/prov101_models.json"),
    )?;
    let manager = SessionManager::new_arc();
    let parent = Uuid::new_v4();
    manager
        .create_session_with_id(
            &parent.to_string(),
            MODEL,
            &directory.path().to_string_lossy(),
            "parent",
        )
        .await?;
    Ok((directory, manager, parent))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_spawn_persists_one_complete_manifest_and_registers_the_subordinate() -> TestResult
{
    let (directory, manager, parent) = setup().await?;
    let handler = create_handler(
        Some(manager.clone()),
        directory.path().to_string_lossy().into_owned(),
        Some(MODEL.into()),
        None,
        None,
    );
    let role = "Read-only reviewer";
    let result = handler(
        AgentManagerAction::Spawn {
            role: Some(role.into()),
        },
        parent,
    );
    let AgentManagerResult::Spawned { session_id } = result else {
        return Err(format!("expected successful spawn, got {result:?}").into());
    };
    let subordinate = Uuid::parse_str(&session_id)?;
    let session = manager.get_session(&session_id)?;
    assert_eq!(session.get_status(), codelet_rpc_types::SessionStatus::Idle);
    assert_eq!(session.get_role().as_deref(), Some(role));
    assert_eq!(manager.get_supervisors(subordinate), vec![parent]);
    assert_eq!(manager.session_count().await, 2);
    let manifest = load_session(subordinate)?;
    assert_eq!(manifest.revision, 1);
    assert_eq!(manifest.provider, MODEL);
    assert_eq!(manifest.project, directory.path());
    assert_eq!(manifest.name, session.get_info().name);
    let cold_store = SessionStore::new()?;
    let durable = cold_store.load(subordinate)?;
    assert_eq!(durable.revision, 1);
    assert_eq!(durable.provider, MODEL);
    assert_eq!(cold_store.list_for_project(directory.path()).len(), 2);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_failed_model_selection_leaves_no_orphan_subordinate_manifest() -> TestResult {
    let (directory, manager, parent) = setup().await?;
    let handler = create_handler(
        Some(manager.clone()),
        directory.path().to_string_lossy().into_owned(),
        Some("malformed-model-without-provider".into()),
        None,
        None,
    );
    let result = handler(AgentManagerAction::Spawn { role: None }, parent);
    assert!(
        matches!(result, AgentManagerResult::Error { ref message, .. } if message.contains("Invalid model string")),
        "expected model selection failure, got {result:?}"
    );
    assert_eq!(manager.session_count().await, 1);
    assert!(manager.get_subordinates(parent).is_empty());
    let store = SessionStore::new()?;
    assert_eq!(store.list_for_project(directory.path()).len(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_manifest_persistence_failure_prevents_subordinate_registration() -> TestResult {
    let (directory, manager, parent) = setup().await?;
    let sessions = directory.path().join("sessions");
    std::fs::rename(&sessions, directory.path().join("saved-sessions"))?;
    std::fs::write(&sessions, "not a directory")?;
    let handler = create_handler(
        Some(manager.clone()),
        directory.path().to_string_lossy().into_owned(),
        Some(MODEL.into()),
        None,
        None,
    );
    let result = handler(AgentManagerAction::Spawn { role: None }, parent);
    assert!(
        matches!(result, AgentManagerResult::Error { ref message, .. } if message.contains("Failed to persist session manifest")),
        "expected persistence failure, got {result:?}"
    );
    assert_eq!(manager.session_count().await, 1);
    assert!(manager.get_subordinates(parent).is_empty());
    Ok(())
}
