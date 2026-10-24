//! Feature: spec/features/session-work-unit-focus.feature

use codelet_core::persistence::{append_message, load_session};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const MODEL: &str = "anthropic/claude-sonnet-4";
static DATA: OnceLock<tempfile::TempDir> = OnceLock::new();

fn fixture() -> Result<(tempfile::TempDir, Arc<SessionManager>), Box<dyn std::error::Error>> {
    if DATA.get().is_none() {
        let _ = DATA.set(setup_temp_directory()?);
    }
    let data = DATA.get().ok_or("test data directory")?;
    let cache = data.path().join("cache");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(
        cache.join("models.json"),
        include_str!("fixtures/rpc422_models.json"),
    )?;
    let _ = codelet_common::set_data_directory(data.path().to_path_buf());
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    let project = setup_temp_directory()?;
    write_cards(project.path(), "testing", true)?;
    Ok((project, SessionManager::new_arc()))
}

fn write_cards(project: &std::path::Path, status: &str, include_second: bool) -> TestResult {
    std::fs::create_dir_all(project.join("spec"))?;
    let mut cards = serde_json::json!({
        "FOCUS-001": {"id":"FOCUS-001", "title":"Current title", "status":status}
    });
    if include_second {
        cards["FOCUS-002"] = serde_json::json!({
            "id":"FOCUS-002", "title":"Second card", "status":"implementing"
        });
    }
    std::fs::write(
        project.join("spec/work-units.json"),
        serde_json::to_vec(&serde_json::json!({"workUnits":cards}))?,
    )?;
    Ok(())
}

async fn create(manager: &SessionManager, project: &std::path::Path) -> Result<SessionId, String> {
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, MODEL, &project.to_string_lossy(), "Work on WRONG-999")
        .await?;
    Ok(SessionId::from(id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_focus_transitions_are_ordered_isolated_and_project_validated(
) -> TestResult {
    // @step Given two live sessions with cards in their own project
    let (project, manager) = fixture()?;
    let first = create(&manager, project.path()).await?;
    let other_project = setup_temp_directory()?;
    let second = create(&manager, other_project.path()).await?;
    let session = manager.get_session(&first.value)?;
    session.set_work_unit_context(
        Some("FOCUS-001".into()),
        Some("bound".into()),
        Some("testing".into()),
    );
    manager.set_session_focus(&first, None, "setup")?;
    *session
        .worktree_path
        .write()
        .map_err(|error| error.to_string())? = Some(project.path().to_path_buf());
    let mut events = manager.work_unit_focus_rx();

    // @step When one session selects a card switches cards and explicitly clears focus
    let selected = manager.set_session_focus(&first, Some("FOCUS-001"), "tool")?;
    assert_eq!(
        selected
            .context
            .as_ref()
            .map(|context| context.title.as_str()),
        Some("Current title")
    );
    assert_eq!(
        manager.set_session_focus(&first, Some("FOCUS-001"), "tool")?,
        selected
    );
    let switched = manager.set_session_focus(&first, Some("FOCUS-002"), "status")?;
    let cleared = manager.set_session_focus(&first, None, "tool")?;

    // @step Then revisions increase only for changes and the other session remains unchanged
    assert_eq!(switched.revision, selected.revision + 1);
    assert_eq!(cleared.revision, switched.revision + 1);
    assert_eq!(cleared.source, "tool");
    assert!(cleared.context.is_none());
    assert_eq!(manager.get_work_unit_focus(&second)?.revision, 0);
    for expected in [selected, switched, cleared.clone()] {
        let (id, actual) = events.try_recv()?;
        assert_eq!(id, first);
        assert_eq!(actual, expected);
    }
    assert!(events.try_recv().is_err());

    // @step And unknown cards are rejected without changing focus
    assert!(manager
        .set_session_focus(&first, Some("FOREIGN-001"), "tool")
        .is_err());
    assert!(manager
        .set_session_focus(&second, Some("FOCUS-001"), "tool")
        .is_err());
    assert_eq!(manager.get_work_unit_focus(&first)?, cleared);

    // @step And clearing activity retains authorization and worktree bindings
    assert_eq!(
        session
            .get_work_unit_context()
            .and_then(|context| context.status),
        Some("testing".into())
    );
    assert_eq!(session.worktree_path(), Some(project.path().to_path_buf()));
    assert_eq!(session.get_info().work_unit_focus, cleared);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_busy_session_reminders_reconcile_the_latest_focus_without_deadlock() -> TestResult
{
    // @step Given a live session whose inner agent lock is held
    let (project, manager) = fixture()?;
    let id = create(&manager, project.path()).await?;
    let session = manager.get_session(&id.value)?;
    let guard = session.inner.lock().await;

    // @step When its focus changes twice and is explicitly cleared
    manager.set_session_focus(&id, Some("FOCUS-001"), "tool")?;
    manager.set_session_focus(&id, Some("FOCUS-002"), "tool")?;
    manager.set_session_focus(&id, None, "tool")?;

    // @step Then the changes return without waiting for the inner lock
    assert!(manager.get_work_unit_focus(&id)?.context.is_none());
    drop(guard);

    // @step And the next environment reminder reflects the latest cleared focus only
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let inner = session.inner.lock().await;
            let text = serde_json::to_string(&inner.messages)?;
            if text.contains("Current work unit: NONE") {
                assert!(!text.contains("Current work unit: FOCUS-001"));
                assert!(!text.contains("Current work unit: FOCUS-002"));
                break;
            }
            drop(inner);
            tokio::task::yield_now().await;
        }
        Ok::<(), serde_json::Error>(())
    })
    .await??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_focus_restore_preserves_history_and_reconciles_missing_or_completed_cards(
) -> TestResult {
    // @step Given persisted sessions with active cleared legacy completed and deleted focus
    let (project, manager) = fixture()?;
    let active = create(&manager, project.path()).await?;
    let cleared = create(&manager, project.path()).await?;
    let legacy = create(&manager, project.path()).await?;
    manager.set_session_focus(&active, Some("FOCUS-001"), "tool")?;
    manager.set_session_focus(&cleared, Some("FOCUS-001"), "tool")?;
    manager.get_session(&cleared.value)?.set_work_unit_context(
        Some("FOCUS-001".into()),
        Some("Current title".into()),
        Some("testing".into()),
    );
    manager.set_session_focus(&cleared, None, "tool")?;
    let completed_project = setup_temp_directory()?;
    write_cards(completed_project.path(), "testing", true)?;
    let completed = create(&manager, completed_project.path()).await?;
    manager.set_session_focus(&completed, Some("FOCUS-001"), "tool")?;
    let deleted = create(&manager, completed_project.path()).await?;
    manager.set_session_focus(&deleted, Some("FOCUS-002"), "tool")?;
    write_cards(completed_project.path(), "done", false)?;
    let mut old = serde_json::to_value(load_session(Uuid::parse_str(&legacy.value)?)?)?;
    old.as_object_mut()
        .ok_or("manifest object")?
        .remove("work_unit_focus");
    old.as_object_mut()
        .ok_or("manifest object")?
        .remove("work_unit_authorization");
    std::fs::write(
        DATA.get()
            .ok_or("data fixture")?
            .path()
            .join("sessions")
            .join(format!("{}.json", legacy.value)),
        serde_json::to_vec(&old)?,
    )?;
    let active_uuid = Uuid::parse_str(&active.value)?;
    let mut stale = load_session(active_uuid)?;
    let writer = std::thread::spawn(move || append_message(&mut stale, "user", "preserved"));
    manager.set_session_focus(&active, Some("FOCUS-002"), "tool")?;
    assert!(writer.join().map_err(|_| "message writer failed")?.is_ok());

    // @step When the real session manager restores their manifests
    let restored = SessionManager::new_arc();
    for id in [&active, &cleared, &legacy, &completed, &deleted] {
        let manifest = load_session(Uuid::parse_str(&id.value)?)?;
        restored
            .create_session_from_manifest(&manifest, MODEL)
            .await?;
    }

    // @step Then active focus uses current card metadata and other focus is explicitly NONE
    let focus = restored.get_work_unit_focus(&active)?;
    assert_eq!(
        focus.context.as_ref().map(|context| context.id.as_str()),
        Some("FOCUS-002")
    );
    for id in [&cleared, &legacy, &completed, &deleted] {
        assert!(restored.get_work_unit_focus(id)?.context.is_none());
        assert!(load_session(Uuid::parse_str(&id.value)?)?
            .work_unit_focus
            .context
            .is_none());
    }

    // @step And concurrent message persistence is preserved
    assert_eq!(load_session(active_uuid)?.messages.len(), 1);

    // @step And clearing activity retains restored authorization bindings
    assert_eq!(
        restored
            .get_session(&cleared.value)?
            .get_work_unit_context()
            .and_then(|context| context.status),
        Some("testing".into())
    );

    // @step And session search reports persisted focus rather than guessing from session names
    let handler = codelet_agent_loop::session_search_handler::create_handler(
        project.path().to_path_buf(),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );
    let result = handler(
        codelet_tools::session_search::types::SessionSearchAction::Recent { count: Some(100) },
        active_uuid,
    );
    let codelet_tools::session_search::SearchResult::Recent { sessions } = result else {
        return Err("expected recent session summaries".into());
    };
    let active_summary = sessions
        .iter()
        .find(|summary| summary.session_id == active.value)
        .ok_or("active summary")?;
    assert_eq!(active_summary.work_unit_id.as_deref(), Some("FOCUS-002"));
    for id in [&cleared, &legacy] {
        let summary = sessions
            .iter()
            .find(|summary| summary.session_id == id.value)
            .ok_or("cleared summary")?;
        assert!(summary.work_unit_id.is_none());
    }
    Ok(())
}
