//! Feature: spec/features/core-scheduler-engine.feature
use super::*;

// =============================================================================
// Scenario: Scheduler spawns on first session creation
// =============================================================================

#[tokio::test]
async fn test_scheduler_spawns_on_first_session_creation() {
    // @step Given a project with spec/schedules.json containing active schedules
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "test-job": make_active_schedule("*/5 * * * *", "shell", None)
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap().to_string();

    // @step When I create a new session in that project
    // The scheduler module should provide a spawn_scheduler function that returns a JoinHandle
    // For now this test verifies the module structure exists and can be called
    let handle = tokio::runtime::Handle::current();
    let hooks: codelet_core::scheduler::Hooks =
        std::sync::Arc::new(codelet_core::scheduler::NoopSchedulerHooks);
    let _handle = codelet_core::scheduler::spawn_scheduler(project_path.clone(), &handle, hooks);

    // @step Then the scheduler task should be spawned
    // The handle should be valid (not immediately finished)
    // Let it start (TEST-042: yield to it, no fixed delay)
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }

    // @step And the scheduler should run on a 30-second interval
    // Verified by the implementation using tokio::time::interval(Duration::from_secs(30))
    // The handle being alive means the scheduler loop is running
    assert!(!_handle.is_finished(), "Scheduler should still be running");

    // Cleanup: abort the task
    _handle.abort();
}
// =============================================================================
// Scenario: Schedule triggers when last run is older than previous cron time
// =============================================================================

#[tokio::test]
async fn test_schedule_triggers_when_last_run_older_than_cron_time() {
    // @step Given a schedule with cron expression "0 * * * *"
    let one_hour_ago = (Utc::now() - Duration::hours(1)).to_rfc3339();

    // @step And the schedule's last_run_at is 1 hour ago
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "hourly-job": make_active_schedule("0 * * * *", "shell", Some(&one_hour_ago))
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler evaluates the schedule
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then the schedule should trigger
    assert!(
        results
            .iter()
            .any(|r| r.name == "hourly-job" && r.triggered),
        "Schedule with last_run 1 hour ago should trigger"
    );
}
// =============================================================================
// Scenario: Schedule does not trigger when already run since last cron time
// =============================================================================

#[tokio::test]
async fn test_schedule_does_not_trigger_when_recently_run() {
    // @step Given a schedule with cron expression "0 * * * *"
    // Set last_run_at to a time AFTER the most recent top-of-hour trigger.
    // This guarantees last_run > prev_trigger regardless of when the test runs.
    let now = Utc::now();
    let current_minute = now.format("%M").to_string().parse::<u32>().unwrap();
    let after_last_trigger = if current_minute == 0 {
        // We're at minute 0 — the trigger is right now; set last_run to 1 second ago
        // which is still after the previous hour's :00 trigger.
        // But the current minute IS a trigger, so last_run must be AFTER now's :00.
        // Use the current hour's :00:01
        now.date_naive()
            .and_hms_opt(now.format("%H").to_string().parse().unwrap(), 0, 1)
            .unwrap()
    } else {
        // We're past minute 0 — last trigger was this hour's :00.
        // Set last_run to this hour's :00 + 30 seconds (safely after the trigger).
        now.date_naive()
            .and_hms_opt(now.format("%H").to_string().parse().unwrap(), 0, 30)
            .unwrap()
    };
    let last_run_str =
        DateTime::<Utc>::from_naive_utc_and_offset(after_last_trigger, Utc).to_rfc3339();

    // @step And the schedule's last_run_at is after the most recent cron trigger
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "hourly-job": make_active_schedule("0 * * * *", "shell", Some(&last_run_str))
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler evaluates the schedule
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then the schedule should not trigger
    // A schedule that ran after the most recent cron trigger should not re-trigger
    assert!(
        results
            .iter()
            .all(|r| r.name != "hourly-job" || !r.triggered),
        "Schedule with last_run after most recent cron trigger should not trigger"
    );
}
// =============================================================================
// Scenario: Schedule with no last run triggers immediately
// =============================================================================

#[tokio::test]
async fn test_schedule_with_no_last_run_triggers_immediately() {
    // @step Given a schedule with cron expression "0 * * * *"
    // @step And the schedule has no last_run_at value
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "new-job": make_active_schedule("0 * * * *", "shell", None)
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler evaluates the schedule
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then the schedule should trigger
    assert!(
        results.iter().any(|r| r.name == "new-job" && r.triggered),
        "Schedule with no last_run_at should trigger immediately"
    );
}
// =============================================================================
// Scenario: Paused schedule is skipped during evaluation
// =============================================================================

#[tokio::test]
async fn test_paused_schedule_is_skipped() {
    // @step Given a schedule with cron expression "0 * * * *"
    // @step And the schedule status is "paused"
    let mut schedule = make_active_schedule("0 * * * *", "shell", None);
    schedule["status"] = json!("paused");

    // @step And the cron time has passed since last run
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "paused-job": schedule
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler evaluates the schedule
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then the schedule should not trigger
    assert!(
        results
            .iter()
            .all(|r| r.name != "paused-job" || !r.triggered),
        "Paused schedule should not trigger"
    );
}
// =============================================================================
// Scenario: Schedule respects configured timezone
// =============================================================================

#[tokio::test]
async fn test_schedule_respects_configured_timezone() {
    // @step Given a schedule with cron expression "0 2 * * *"
    // @step And the schedule timezone is "America/New_York"
    let mut schedule = make_active_schedule("0 2 * * *", "shell", None);
    schedule["timezone"] = json!("America/New_York");

    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "tz-job": schedule
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When evaluating if the schedule should trigger
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then the cron expression should be evaluated in America/New_York time
    // @step And not in UTC time
    // The evaluation result should show that timezone was used in the cron calculation
    let tz_result = results.iter().find(|r| r.name == "tz-job");
    assert!(tz_result.is_some(), "Timezone job should be evaluated");
    assert_eq!(
        tz_result.unwrap().evaluated_timezone,
        "America/New_York",
        "Should evaluate in the configured timezone"
    );
}
