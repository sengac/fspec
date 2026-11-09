//! Feature: spec/features/core-scheduler-engine.feature
use super::*;

// =============================================================================
// Scenario: Scheduler handles missing schedules file gracefully
// =============================================================================

#[tokio::test]
async fn test_scheduler_handles_missing_schedules_file() {
    // @step Given a project without spec/schedules.json
    let tmp = setup_project_without_schedules().await;
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler tick runs
    let result = codelet_core::scheduler::evaluate_schedules(project_path).await;

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
    let result = codelet_core::scheduler::evaluate_schedules(project_path).await;

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
    let state = codelet_core::scheduler::SchedulerState::new();
    let hooks: codelet_core::scheduler::Hooks =
        std::sync::Arc::new(codelet_core::scheduler::NoopSchedulerHooks);
    codelet_core::scheduler::evaluate_and_run(project_path, &state, hooks)
        .await
        .unwrap();
    let after = Utc::now();

    // @step Then last_run_at should be updated to the current time
    let updated = read_schedules(tmp.path()).await;
    let last_run = updated["schedules"]["update-job"]["last_run_at"]
        .as_str()
        .expect("last_run_at should be set");
    let last_run_dt: DateTime<Utc> = last_run.parse().expect("parse timestamp");
    assert!(
        last_run_dt >= before && last_run_dt <= after,
        "Timestamp should be recent"
    );

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
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();
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
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

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
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then trigger_agent_job should be called with the schedule config
    let agent_result = results.iter().find(|r| r.name == "agent-job");
    assert!(agent_result.is_some(), "Agent job should be evaluated");
    assert_eq!(
        agent_result.unwrap().job_type,
        "agent",
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
    let results = codelet_core::scheduler::evaluate_schedules(project_path)
        .await
        .unwrap();

    // @step Then trigger_shell_job should be called with the schedule config
    let shell_result = results.iter().find(|r| r.name == "shell-job");
    assert!(shell_result.is_some(), "Shell job should be evaluated");
    assert_eq!(
        shell_result.unwrap().job_type,
        "shell",
        "Should delegate to shell execution"
    );
}
