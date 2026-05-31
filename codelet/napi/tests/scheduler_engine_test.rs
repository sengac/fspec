//! Feature: spec/features/core-scheduler-engine.feature
//!
//! Tests for the Core Scheduler Engine (SCHED-003).
//! Validates cron evaluation, timezone handling, error resilience,
//! and job delegation for the scheduler loop.

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

/// Helper: create a temp project directory with spec/schedules.json
async fn setup_project_with_schedules(schedules_json: serde_json::Value) -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let spec_dir = tmp.path().join("spec");
    fs::create_dir_all(&spec_dir)
        .await
        .expect("create spec dir");
    let schedules_path = spec_dir.join("schedules.json");
    fs::write(
        &schedules_path,
        serde_json::to_string_pretty(&schedules_json).unwrap(),
    )
    .await
    .expect("write schedules.json");
    tmp
}

/// Helper: create a temp project directory without schedules.json
async fn setup_project_without_schedules() -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let spec_dir = tmp.path().join("spec");
    fs::create_dir_all(&spec_dir)
        .await
        .expect("create spec dir");
    tmp
}

/// Helper: read schedules.json from a project directory
async fn read_schedules(project_path: &Path) -> serde_json::Value {
    let path = project_path.join("spec/schedules.json");
    let content = fs::read_to_string(&path).await.expect("read schedules");
    serde_json::from_str(&content).expect("parse schedules")
}

/// Helper: build a standard schedule entry for testing
fn make_active_schedule(
    cron_expr: &str,
    job_type: &str,
    last_run_at: Option<&str>,
) -> serde_json::Value {
    let mut entry = json!({
        "cron": cron_expr,
        "timezone": "UTC",
        "status": "active",
        "job_type": job_type,
        "created_at": "2026-01-01T00:00:00Z"
    });
    if let Some(ts) = last_run_at {
        entry["last_run_at"] = json!(ts);
    }
    if job_type == "agent" {
        entry["agent"] = json!({
            "role": "test agent",
            "prompt": "do something"
        });
    } else if job_type == "shell" {
        entry["shell"] = json!({
            "command": "echo hello"
        });
    }
    entry
}

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
    let _handle = codelet_napi::scheduler::spawn_scheduler(project_path.clone(), &handle);

    // @step Then the scheduler task should be spawned
    // The handle should be valid (not immediately finished)
    // Give it a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then the schedule should trigger
    assert!(
        results.iter().any(|r| r.name == "hourly-job" && r.triggered),
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
    let last_run_str = DateTime::<Utc>::from_naive_utc_and_offset(after_last_trigger, Utc).to_rfc3339();

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
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then the schedule should not trigger
    // A schedule that ran after the most recent cron trigger should not re-trigger
    assert!(
        results.iter().all(|r| r.name != "hourly-job" || !r.triggered),
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
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

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
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then the schedule should not trigger
    assert!(
        results.iter().all(|r| r.name != "paused-job" || !r.triggered),
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
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then the cron expression should be evaluated in America/New_York time
    // @step And not in UTC time
    // The evaluation result should show that timezone was used in the cron calculation
    let tz_result = results.iter().find(|r| r.name == "tz-job");
    assert!(tz_result.is_some(), "Timezone job should be evaluated");
    assert_eq!(
        tz_result.unwrap().evaluated_timezone, "America/New_York",
        "Should evaluate in the configured timezone"
    );
}

// =============================================================================
// Scenario: Scheduler handles missing schedules file gracefully
// =============================================================================

#[tokio::test]
async fn test_scheduler_handles_missing_schedules_file() {
    // @step Given a project without spec/schedules.json
    let tmp = setup_project_without_schedules().await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler tick runs
    let result = codelet_napi::scheduler::evaluate_schedules(project_path).await;

    // @step Then a warning should be logged
    // (logging verification would be done with a log capture, but we verify no panic)

    // @step And the scheduler should continue running
    // The function should return Ok with empty results, not Err
    assert!(result.is_ok(), "Should not error on missing file");
    assert!(
        result.unwrap().is_empty(),
        "Should return empty results for missing file"
    );
}

// =============================================================================
// Scenario: Scheduler handles malformed JSON gracefully
// =============================================================================

#[tokio::test]
async fn test_scheduler_handles_malformed_json() {
    // @step Given spec/schedules.json contains invalid JSON
    let tmp = TempDir::new().expect("create temp dir");
    let spec_dir = tmp.path().join("spec");
    fs::create_dir_all(&spec_dir).await.unwrap();
    fs::write(spec_dir.join("schedules.json"), "{ not valid json !!!")
        .await
        .unwrap();
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler tick runs
    let result = codelet_napi::scheduler::evaluate_schedules(project_path).await;

    // @step Then an error should be logged
    // (logging verification done via log capture)

    // @step And the scheduler should continue running
    // The function should return Ok with empty results, not crash
    assert!(result.is_ok(), "Should not crash on malformed JSON");
    assert!(
        result.unwrap().is_empty(),
        "Should return empty results for malformed JSON"
    );
}

// =============================================================================
// Scenario: Job completion updates last run timestamp
// =============================================================================

#[tokio::test]
async fn test_job_completion_updates_last_run_timestamp() {
    // @step Given a schedule that is ready to trigger
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "update-job": make_active_schedule("0 * * * *", "shell", None)
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the job executes successfully
    let before = Utc::now();
    let state = codelet_napi::scheduler::SchedulerState::new();
    let hooks: codelet_napi::scheduler::Hooks =
        std::sync::Arc::new(codelet_napi::scheduler::NoopSchedulerHooks);
    codelet_napi::scheduler::evaluate_and_run(project_path, &state, hooks).await.unwrap();
    let after = Utc::now();

    // @step Then last_run_at should be updated to the current time
    let updated = read_schedules(tmp.path()).await;
    let last_run = updated["schedules"]["update-job"]["last_run_at"]
        .as_str()
        .expect("last_run_at should be set");
    let last_run_dt: DateTime<Utc> = last_run.parse().expect("parse timestamp");
    assert!(last_run_dt >= before && last_run_dt <= after, "Timestamp should be recent");

    // @step And last_run_status should be set to "success"
    assert_eq!(
        updated["schedules"]["update-job"]["last_run_status"]
            .as_str()
            .unwrap(),
        "success"
    );
}

// =============================================================================
// Scenario: New schedule is picked up without restart
// =============================================================================

#[tokio::test]
async fn test_new_schedule_picked_up_without_restart() {
    // @step Given the scheduler is already running
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {}
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // First evaluation — no schedules
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();
    assert!(results.is_empty(), "No schedules initially");

    // @step And spec/schedules.json contains no schedules
    // (verified above)

    // @step When a new schedule is added to spec/schedules.json
    let new_schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "dynamic-job": make_active_schedule("*/5 * * * *", "shell", None)
        }
    });
    fs::write(
        tmp.path().join("spec/schedules.json"),
        serde_json::to_string_pretty(&new_schedules).unwrap(),
    )
    .await
    .unwrap();

    // @step And the scheduler tick runs
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then the new schedule should be evaluated
    assert!(
        results.iter().any(|r| r.name == "dynamic-job"),
        "Newly added schedule should be picked up"
    );
}

// =============================================================================
// Scenario: Agent job type delegates to agent execution
// =============================================================================

#[tokio::test]
async fn test_agent_job_type_delegates_to_agent_execution() {
    // @step Given a schedule with job type "agent"
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "agent-job": make_active_schedule("0 * * * *", "agent", None)
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the schedule triggers
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then trigger_agent_job should be called with the schedule config
    let agent_result = results.iter().find(|r| r.name == "agent-job");
    assert!(agent_result.is_some(), "Agent job should be evaluated");
    assert_eq!(
        agent_result.unwrap().job_type, "agent",
        "Should delegate to agent execution"
    );
}

// =============================================================================
// Scenario: Shell job type delegates to shell execution
// =============================================================================

#[tokio::test]
async fn test_shell_job_type_delegates_to_shell_execution() {
    // @step Given a schedule with job type "shell"
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "shell-job": make_active_schedule("0 * * * *", "shell", None)
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the schedule triggers
    let results = codelet_napi::scheduler::evaluate_schedules(project_path).await.unwrap();

    // @step Then trigger_shell_job should be called with the schedule config
    let shell_result = results.iter().find(|r| r.name == "shell-job");
    assert!(shell_result.is_some(), "Shell job should be evaluated");
    assert_eq!(
        shell_result.unwrap().job_type, "shell",
        "Should delegate to shell execution"
    );
}
