//! Feature: spec/features/schedule-ai-tool.feature
use super::*;

// =============================================================================
// Scenario: Add an agent-type schedule
// =============================================================================

#[tokio::test]
async fn test_add_agent_type_schedule() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add", name "nightly-review", cron "0 2 * * *", timezone "Australia/Sydney", job_type "agent", role "Code reviewer", and prompt "Review recent changes"
    let req = add_agent_request(
        "nightly-review",
        "0 2 * * *",
        "Australia/Sydney",
        "Code reviewer",
        "Review recent changes",
    );
    let result = handler(req);

    // @step Then the response should have success true and action "add"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("add"));

    // @step And the response schedule should have name "nightly-review", cron "0 2 * * *", timezone "Australia/Sydney", and job_type "agent"
    let schedule = result.schedule.expect("Expected schedule in response");
    assert_eq!(schedule["name"], "nightly-review");
    assert_eq!(schedule["cron"], "0 2 * * *");
    assert_eq!(schedule["timezone"], "Australia/Sydney");
    assert_eq!(
        schedule["jobType"]
            .as_str()
            .or(schedule["job_type"].as_str()),
        Some("agent")
    );

    // @step And schedules.json should contain a schedule named "nightly-review"
    let schedules = read_schedules(tmp.path()).await;
    assert!(
        schedules.get("nightly-review").is_some(),
        "Schedule 'nightly-review' not found in schedules.json"
    );
}
// =============================================================================
// Scenario: Add a shell-type schedule
// =============================================================================

#[tokio::test]
async fn test_add_shell_type_schedule() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add", name "daily-lint", cron "0 6 * * 1-5", timezone "UTC", job_type "shell", and command "npm run lint"
    let req = add_shell_request("daily-lint", "0 6 * * 1-5", "UTC", "npm run lint");
    let result = handler(req);

    // @step Then the response should have success true and action "add"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("add"));

    // @step And the response schedule should have name "daily-lint" and job_type "shell"
    let schedule = result.schedule.expect("Expected schedule in response");
    assert_eq!(schedule["name"], "daily-lint");
    assert_eq!(
        schedule["jobType"]
            .as_str()
            .or(schedule["job_type"].as_str()),
        Some("shell")
    );

    // @step And schedules.json should contain a schedule named "daily-lint"
    let schedules = read_schedules(tmp.path()).await;
    assert!(
        schedules.get("daily-lint").is_some(),
        "Schedule 'daily-lint' not found in schedules.json"
    );
}
// =============================================================================
// Scenario: List all schedules
// =============================================================================

#[tokio::test]
async fn test_list_all_schedules() {
    // @step Given a schedule named "nightly-review" exists with cron "0 2 * * *" and type "agent"
    // @step And a schedule named "daily-sync" exists with cron "0 9 * * 1-5" and type "shell"
    let tmp = setup_project_with_schedules(json!({
        "nightly-review": agent_entry("0 2 * * *", "UTC", "active"),
        "daily-sync": shell_entry("0 9 * * 1-5", "UTC", "active")
    }))
    .await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "list"
    let result = handler(list_request());

    // @step Then the response should have success true and action "list"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("list"));

    // @step And the response should contain 2 schedules with names "nightly-review" and "daily-sync"
    let schedules = result
        .schedules
        .expect("Expected schedules list in response");
    assert_eq!(
        schedules.len(),
        2,
        "Expected 2 schedules, got {}",
        schedules.len()
    );

    let names: Vec<String> = schedules
        .iter()
        .filter_map(|s| s["name"].as_str().map(String::from))
        .collect();
    assert!(
        names.contains(&"nightly-review".to_string()),
        "Missing nightly-review"
    );
    assert!(
        names.contains(&"daily-sync".to_string()),
        "Missing daily-sync"
    );
}
// =============================================================================
// Scenario: Pause an active schedule
// =============================================================================

#[tokio::test]
async fn test_pause_active_schedule() {
    // @step Given a schedule named "nightly-review" exists with status "active"
    let tmp = setup_project_with_schedules(json!({
        "nightly-review": agent_entry("0 2 * * *", "UTC", "active")
    }))
    .await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "pause" and name "nightly-review"
    let result = handler(action_request("pause", "nightly-review"));

    // @step Then the response should have success true and action "pause"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("pause"));

    // @step And schedules.json should show "nightly-review" with status "paused"
    let schedules = read_schedules(tmp.path()).await;
    assert_eq!(
        schedules["nightly-review"]["status"], "paused",
        "Expected status 'paused', got {:?}",
        schedules["nightly-review"]["status"]
    );
}
// =============================================================================
// Scenario: Resume a paused schedule
// =============================================================================

#[tokio::test]
async fn test_resume_paused_schedule() {
    // @step Given a schedule named "nightly-review" exists with status "paused"
    let tmp = setup_project_with_schedules(json!({
        "nightly-review": agent_entry("0 2 * * *", "UTC", "paused")
    }))
    .await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "resume" and name "nightly-review"
    let result = handler(action_request("resume", "nightly-review"));

    // @step Then the response should have success true and action "resume"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("resume"));

    // @step And schedules.json should show "nightly-review" with status "active"
    let schedules = read_schedules(tmp.path()).await;
    assert_eq!(
        schedules["nightly-review"]["status"], "active",
        "Expected status 'active', got {:?}",
        schedules["nightly-review"]["status"]
    );
}
// =============================================================================
// Scenario: Remove an existing schedule
// =============================================================================

#[tokio::test]
async fn test_remove_existing_schedule() {
    // @step Given a schedule named "nightly-review" exists
    let tmp = setup_project_with_schedules(json!({
        "nightly-review": agent_entry("0 2 * * *", "UTC", "active")
    }))
    .await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "remove" and name "nightly-review"
    let result = handler(action_request("remove", "nightly-review"));

    // @step Then the response should have success true and action "remove"
    assert!(
        result.success,
        "Expected success, got error: {:?}",
        result.error
    );
    assert_eq!(result.action.as_deref(), Some("remove"));

    // @step And schedules.json should not contain a schedule named "nightly-review"
    let schedules = read_schedules(tmp.path()).await;
    assert!(
        schedules.get("nightly-review").is_none(),
        "Schedule 'nightly-review' should have been removed"
    );
}
