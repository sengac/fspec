//! Feature: spec/features/session-status-focus-reconciliation.feature
pub mod bug231_support;
use bug231_support::{Fixture, Result};
use codelet_core::{
    persistence::load_session, session_manager_handle::SessionManagerHandle,
    work_units::WorkUnitsWatcher,
};
use codelet_tools::execution_context::{scope_invocation, WorkUnitAttribution};
use std::{sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_successful_legacy_agent_status_command_switches_the_issuing_session() -> Result
{
    // @step Given two sessions with different card activity
    let fixture = Fixture::new().await?;
    let second = fixture.manager.get_work_unit_focus(&fixture.second)?;
    // @step When the background session successfully updates another card without explicit attribution
    fixture.status("AUTH-003", "validating").await?;
    // @step Then only that issuing session switches to the updated card
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-003"));
    assert_eq!(
        fixture.manager.get_work_unit_focus(&fixture.second)?,
        second
    );
    // @step And its environment reminder and current card status agree
    let session = fixture.manager.get_session(&fixture.first.value)?;
    let text = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert!(text.contains("Current work unit: AUTH-003"));
    assert_eq!(
        session.activity_focus().context.ok_or("focus")?.status,
        "validating"
    );
    bug231_support::agent_turn::verify(&fixture).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_a_failed_status_command_does_not_imply_a_successful_focus_switch() -> Result {
    // @step Given a session working on card "AUTH-001"
    let fixture = Fixture::new().await?;
    let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
    let mut events = fixture.manager.work_unit_focus_rx();
    // @step When its attempted transition for card "AUTH-002" fails
    assert!(fixture.status("AUTH-002", "done").await.is_err());
    // @step Then its activity remains on "AUTH-001"
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
    // @step And no focus change event is emitted for that failed command
    assert!(events.try_recv().is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_same_card_status_refresh_retains_correct_authorization_stage() -> Result {
    // @step Given a session with activity and authorization bound to card "AUTH-001"
    let fixture = Fixture::new().await?;
    let session = fixture.manager.get_session(&fixture.first.value)?;
    session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("old".into()),
        Some("implementing".into()),
    );
    codelet_sessions::session_tool_callbacks::register_manager(fixture.manager.clone());
    // @step When a successful status update changes that card's stage
    fixture.status("AUTH-001", "validating").await?;
    // @step Then the authoritative activity contains the current stage
    assert_eq!(
        session.activity_focus().context.ok_or("focus")?.status,
        "validating"
    );
    // @step And existing stage checks use the current stage even after activity is cleared
    fixture
        .manager
        .set_session_focus(&fixture.first, None, "clear")?;
    fixture.shell("AUTH-001", "specifying")?;
    fixture.observe()?;
    assert!(fixture.focus(&fixture.first)?.is_none());
    assert_eq!(
        codelet_sessions::session_tool_callbacks::work_unit_stage(fixture.first.value.clone())
            .as_deref(),
        Some("specifying")
    );
    bug231_support::metadata::verify_callback_lock(&fixture).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_external_completion_or_deletion_clears_only_sessions_focused_on_that_card(
) -> Result {
    // @step Given two sessions focused on "AUTH-001" and another on "AUTH-002"
    let fixture = Fixture::new().await?;
    let third = bug231_support::create(&fixture.manager, fixture.project.path()).await?;
    fixture
        .manager
        .set_session_focus(&third, Some("AUTH-001"), "setup")?;
    let session = fixture.manager.get_session(&fixture.first.value)?;
    session.set_work_unit_context(
        Some("AUTH-001".into()),
        Some("bound".into()),
        Some("testing".into()),
    );
    let watcher = Arc::new(WorkUnitsWatcher::new(fixture.project.path())?);
    let _subscription = fixture.manager.watch_work_units(&watcher);
    // @step When a real work-unit snapshot marks "AUTH-001" completed or removes it
    fixture.shell("AUTH-001", "validating")?;
    fixture.shell("AUTH-001", "done")?;
    // @step Then both sessions on "AUTH-001" remain open with cleared activity
    fixture.wait_focus(&fixture.first, None).await?;
    fixture.wait_focus(&third, None).await?;
    assert!(fixture.manager.get_session(&third.value).is_ok());
    // @step And the session on "AUTH-002" remains unchanged
    assert_eq!(fixture.focus(&fixture.second)?.as_deref(), Some("AUTH-002"));
    // @step And permission bindings are not relaxed by clearing activity
    assert_eq!(
        session
            .get_work_unit_context()
            .and_then(|context| context.status)
            .as_deref(),
        Some("done")
    );
    bug231_support::metadata::bind_before_deletion(&fixture, &session)?;
    let result = codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "delete-work-unit".into(),
        args_json: serde_json::json!({"workUnitId":"AUTH-002"}).to_string(),
        project_root: fixture.project.path().to_owned(),
    });
    assert!(result.success, "{:?}", result.error);
    fixture.wait_focus(&fixture.first, None).await?;
    fixture.wait_focus(&fixture.second, None).await?;
    bug231_support::metadata::assert_deleted_binding(&session);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_explicit_attribution_takes_precedence_over_administrative_command_targets(
) -> Result {
    // @step Given a session with explicit tool attribution for "AUTH-001" or "NONE"
    let fixture = Fixture::new().await?;
    for attribution in [
        WorkUnitAttribution::Card("AUTH-001".into()),
        WorkUnitAttribution::None,
    ] {
        let receipt = fixture
            .manager
            .admit_tool_invocation(&fixture.first, attribution)?;
        let before = fixture.manager.get_work_unit_focus(&fixture.first)?;
        // @step When it successfully updates another card's status
        scope_invocation(receipt, fixture.status("AUTH-002", "validating")).await?;
        // @step Then the command target does not override its explicit activity attribution
        assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
        // @step And completing an unrelated card does not clear its current activity
        fixture.shell("AUTH-002", "done")?;
        fixture.observe()?;
        assert_eq!(fixture.manager.get_work_unit_focus(&fixture.first)?, before);
        fixture.shell("AUTH-002", "implementing")?;
    }
    bug231_support::callback::verify(&fixture).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_late_completion_cannot_erase_newer_activity_or_forbid_later_follow_up_work(
) -> Result {
    // @step Given a session that has moved to newer activity after starting a card transition
    let fixture = Fixture::new().await?;
    let old = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Absent)?;
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-002"), "tool")?;
    // @step When the older card's completion is reconciled
    fixture.shell("AUTH-001", "validating")?;
    fixture.shell("AUTH-001", "done")?;
    fixture.manager.reconcile_status_success(&old, "AUTH-001")?;
    // @step Then the newer activity is preserved
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-002"));
    // @step And an explicitly attributed follow-up call may select an already completed card
    let followup = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Card("AUTH-001".into()))?;
    let repeated = fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Card("AUTH-001".into()))?;
    assert!(repeated.admission > followup.admission);
    assert_eq!(repeated.focus_revision, followup.focus_revision);
    fixture.observe()?;
    fixture.manager.reconcile_status_success(&old, "AUTH-001")?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    bug231_support::metadata::verify_metadata_race(&fixture)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_sessionless_commands_refresh_metadata_without_claiming_another_card() -> Result {
    // @step Given sessions focused on different cards
    let fixture = Fixture::new().await?;
    let watcher = Arc::new(WorkUnitsWatcher::new(fixture.project.path())?);
    let _subscription = fixture.manager.watch_work_units(&watcher);
    // @step When a shell user updates an unrelated card without a session identity
    fixture.shell("AUTH-003", "validating")?;
    // @step Then no session switches to that card
    fixture.observe()?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    assert_eq!(fixture.focus(&fixture.second)?.as_deref(), Some("AUTH-002"));
    // @step And updates to an already focused card still refresh its metadata
    fixture.shell("AUTH-001", "validating")?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if fixture
                .manager
                .get_work_unit_focus(&fixture.first)?
                .context
                .ok_or("focus")?
                .status
                == "validating"
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_watcher_reconciliation_updates_a_busy_session_outside_the_runtime() -> Result {
    // @step Given a focused session whose agent lock is busy
    let fixture = Fixture::new().await?;
    let session = fixture.manager.get_session(&fixture.first.value)?;
    let guard = session.inner.lock().await;
    let mut events = fixture.manager.work_unit_focus_rx();
    fixture.shell("AUTH-001", "validating")?;
    fixture.shell("AUTH-001", "done")?;
    // @step When a watcher thread outside Tokio observes completion
    let manager = fixture.manager.clone();
    let path = fixture.project.path().to_owned();
    std::thread::spawn(move || manager.reconcile_work_units(&path))
        .join()
        .map_err(|_| "watcher panicked")??;
    // @step Then focus persistence and events complete without waiting for the agent lock
    assert!(fixture.focus(&fixture.first)?.is_none());
    assert!(events.try_recv()?.1.context.is_none());
    assert!(load_session(uuid::Uuid::parse_str(&fixture.first.value)?)?
        .work_unit_focus
        .context
        .is_none());
    drop(guard);
    // @step And the deferred environment reminder reflects cleared activity
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if serde_json::to_string(&session.inner.lock().await.messages)?
                .contains("Current work unit: NONE")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<(), serde_json::Error>(())
    })
    .await??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_watcher_replacement_and_lag_recovery_retain_reconciliation() -> Result {
    // @step Given a session service with a subscribed work-unit watcher
    let fixture = Fixture::new().await?;
    let old = Arc::new(WorkUnitsWatcher::new(fixture.project.path())?);
    let service =
        codelet_rpc::SharedFspecService::with_session_manager(old.clone(), fixture.manager.clone());
    // @step When its watcher is replaced and notifications are missed
    fixture.shell("AUTH-001", "validating")?;
    fixture.shell("AUTH-001", "done")?;
    service.rebuild_watcher(WorkUnitsWatcher::new(fixture.project.path())?);
    // @step Then authoritative resynchronization clears completed activity
    fixture.wait_focus(&fixture.first, None).await?;
    // @step And stopped subscriptions cannot mutate newer activity
    fixture
        .manager
        .admit_tool_invocation(&fixture.first, WorkUnitAttribution::Card("AUTH-001".into()))?;
    fixture.shell("AUTH-003", "validating")?;
    fixture.observe()?;
    assert_eq!(fixture.focus(&fixture.first)?.as_deref(), Some("AUTH-001"));
    drop(service);
    bug231_support::observer::verify(&fixture).await?;
    Ok(())
}
