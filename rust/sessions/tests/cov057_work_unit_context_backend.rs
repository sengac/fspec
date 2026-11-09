//! Feature: spec/features/work-unit-context-backend.feature
//!
//! COV-057: the work-unit context now lives in the session's activity focus
//! (`SessionManager::set_session_focus`, rust/sessions/src/activity_focus.rs),
//! whose environment reminder carries `Current work unit: <ID>` only. Reuses
//! the BUG-231 fixture (real manager, real work-units.json, real fspec tool
//! facade) from bug231_support.

#[path = "bug231_support/mod.rs"]
pub mod bug231_support;

use bug231_support::{Fixture, Result};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::WorkUnitContext;
use serde_json::json;

const TITLE: &str = "Implement OAuth login flow";

/// The newest environment reminder of a session that names a work unit.
async fn environment_reminder(
    fixture: &Fixture,
    session_id: &codelet_rpc_types::SessionId,
) -> Result<String> {
    let session = fixture.manager.get_session(&session_id.value)?;
    let reminder = codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            let found = session
                .inner
                .lock()
                .await
                .messages
                .iter()
                .rev()
                .filter_map(|message| serde_json::to_string(message).ok())
                .find(|text| text.contains("Current work unit:"));
            if let Some(text) = found.filter(|text| text.contains("Current work unit: AUTH-001")) {
                return text;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?;
    Ok(reminder)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_work_unit_id_appears_in_environment_information_when_entering_agentview() -> Result
{
    let fixture = Fixture::new().await?;
    // @step Given work unit "AUTH-001" exists in the backlog
    let project = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    std::fs::write(
        project.path().join("spec/work-units.json"),
        serde_json::to_vec(&json!({
            "workUnits": {"AUTH-001": {"id":"AUTH-001","title":TITLE,"type":"story","status":"backlog"}},
            "states": {"backlog":["AUTH-001"]}
        }))?,
    )?;
    let session_id = bug231_support::create(&fixture.manager, project.path()).await?;
    let handle: &dyn SessionManagerHandle = &*fixture.manager;
    // @step When I select work unit "AUTH-001" and press Enter
    // (the TUI attach path: set_active_session + set_work_unit_context)
    handle.set_active_session(&session_id)?;
    handle.set_work_unit_context(
        &session_id,
        Some(WorkUnitContext {
            id: "AUTH-001".into(),
            title: TITLE.into(),
            status: "backlog".into(),
        }),
    )?;
    // @step Then I should be in the AgentView
    assert_eq!(handle.get_active_session(), Ok(Some(session_id.clone())));
    let reminder = environment_reminder(&fixture, &session_id).await?;
    // @step And the environment information should contain "Current work unit: AUTH-001"
    assert!(
        reminder.contains("Current work unit: AUTH-001"),
        "{reminder}"
    );
    // @step And the environment information should not contain the work unit title
    assert!(!reminder.contains(TITLE), "{reminder}");
    // @step And the environment information should not contain the work unit status
    assert!(!reminder.contains("backlog"), "{reminder}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_no_notification_when_updating_the_same_work_unit() -> Result {
    // @step Given the session is attached to work unit "AUTH-001"
    let fixture = Fixture::new().await?;
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    let mut events = fixture.manager.work_unit_focus_rx();
    // @step When I run "update-work-unit-status AUTH-001 implementing"
    fixture.status("AUTH-001", "implementing").await?;
    // @step Then the session work unit context should remain "AUTH-001"
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    assert!(
        events.try_recv().is_err(),
        "same-card update notified a focus change"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_llm_receives_notification_when_updating_a_different_work_unit() -> Result {
    // @step Given the session is attached to work unit "AUTH-001"
    let fixture = Fixture::new().await?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    let cards = fixture.project.path().join("spec/work-units.json");
    let mut board: serde_json::Value = serde_json::from_slice(&std::fs::read(&cards)?)?;
    board["workUnits"]["BUG-002"] =
        json!({"id":"BUG-002","title":"BUG-002","type":"task","status":"specifying"});
    board["states"]["specifying"] = json!(["BUG-002"]);
    std::fs::write(&cards, serde_json::to_vec(&board)?)?;
    let mut events = fixture.manager.work_unit_focus_rx();
    // @step When I run "update-work-unit-status BUG-002 implementing"
    let output = fixture.status("BUG-002", "implementing").await?;
    // @step Then the session work unit context should be updated to "BUG-002"
    assert_eq!(
        fixture.focus(&fixture.first)?.as_deref(),
        Some("BUG-002"),
        "{output}"
    );
    let (notified, focus) = events.try_recv()?;
    assert_eq!(notified, fixture.first);
    assert_eq!(
        focus.context.map(|context| context.id).as_deref(),
        Some("BUG-002")
    );
    let session = fixture.manager.get_session(&fixture.first.value)?;
    let history = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert!(history.contains("Current work unit: BUG-002"), "{history}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_no_notification_when_no_active_session_exists() -> Result {
    let fixture = Fixture::new().await?;
    let handle: &dyn SessionManagerHandle = &*fixture.manager;
    // @step Given there is no active TUI session
    handle.clear_active_session()?;
    assert_eq!(handle.get_active_session(), Ok(None));
    let before = [
        fixture.manager.get_work_unit_focus(&fixture.first)?,
        fixture.manager.get_work_unit_focus(&fixture.second)?,
    ];
    let mut events = fixture.manager.work_unit_focus_rx();
    fixture.shell("AUTH-003", "validating")?;
    // @step Then context should not be set
    let after = [
        fixture.manager.get_work_unit_focus(&fixture.first)?,
        fixture.manager.get_work_unit_focus(&fixture.second)?,
    ];
    assert_eq!(after, before, "a session-less status update set a context");
    assert!(events.try_recv().is_err(), "a focus change was notified");
    Ok(())
}
