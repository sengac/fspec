//! Feature: spec/features/schedule-ai-tool.feature
use super::*;

// =============================================================================
// Scenario: Reject invalid cron expression
// =============================================================================

#[tokio::test]
async fn test_reject_invalid_cron_expression() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add", name "bad", cron "not-a-cron", timezone "UTC", and job_type "shell"
    let req = ScheduleRequest {
        action: "add".to_string(),
        name: Some("bad".to_string()),
        cron: Some("not-a-cron".to_string()),
        timezone: Some("UTC".to_string()),
        job_type: Some("shell".to_string()),
        role: None,
        prompt: None,
        command: Some("echo test".to_string()),
        overlap_policy: None,
    };
    let result = handler(req);

    // @step Then the response should have success false
    assert!(!result.success, "Expected failure for invalid cron");

    // @step And the error message should contain "Invalid cron expression"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("Invalid cron expression") || err.contains("invalid cron"),
        "Error should mention invalid cron, got: {}",
        err
    );
}
// =============================================================================
// Scenario: Reject duplicate schedule name
// =============================================================================

#[tokio::test]
async fn test_reject_duplicate_schedule_name() {
    // @step Given a schedule named "existing-schedule" exists
    let tmp = setup_project_with_schedules(json!({
        "existing-schedule": shell_entry("0 9 * * *", "UTC", "active")
    }))
    .await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add" and name "existing-schedule" with valid parameters
    let req = add_shell_request("existing-schedule", "0 12 * * *", "UTC", "echo test");
    let result = handler(req);

    // @step Then the response should have success false
    assert!(!result.success, "Expected failure for duplicate name");

    // @step And the error message should contain "Schedule already exists"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("Schedule already exists") || err.contains("already exists"),
        "Error should mention duplicate, got: {}",
        err
    );
}
// =============================================================================
// Scenario: Reject removal of nonexistent schedule
// =============================================================================

#[tokio::test]
async fn test_reject_removal_of_nonexistent_schedule() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "remove" and name "nonexistent"
    let result = handler(action_request("remove", "nonexistent"));

    // @step Then the response should have success false
    assert!(!result.success, "Expected failure for nonexistent schedule");

    // @step And the error message should contain "Schedule not found"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("Schedule not found") || err.contains("not found"),
        "Error should mention not found, got: {}",
        err
    );
}
// =============================================================================
// Scenario: Graceful error when no handler is registered
// =============================================================================

#[tokio::test]
async fn test_graceful_error_when_no_handler_registered() {
    // @step Given a session with no registered schedule handler
    let session_id = uuid::Uuid::new_v4();

    // Ensure no handler is registered
    clear_all_schedule_handlers();
    assert!(!has_schedule_handler(session_id));

    // @step When execute_schedule_command is called for that session
    let req = list_request();
    let result = execute_schedule_command(session_id, req);

    // @step Then the response should have success false
    assert!(
        !result.success,
        "Expected failure when no handler registered"
    );

    // @step And the error message should contain "No schedule handler registered"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("No schedule handler registered"),
        "Error should mention no handler, got: {}",
        err
    );
}
// =============================================================================
// Scenario: Reject invalid timezone
// =============================================================================

#[tokio::test]
async fn test_reject_invalid_timezone() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add", name "test", cron "0 9 * * *", timezone "Invalid/Zone", and job_type "shell"
    let req = ScheduleRequest {
        action: "add".to_string(),
        name: Some("test".to_string()),
        cron: Some("0 9 * * *".to_string()),
        timezone: Some("Invalid/Zone".to_string()),
        job_type: Some("shell".to_string()),
        role: None,
        prompt: None,
        command: Some("echo test".to_string()),
        overlap_policy: None,
    };
    let result = handler(req);

    // @step Then the response should have success false
    assert!(!result.success, "Expected failure for invalid timezone");

    // @step And the error message should contain "Invalid timezone"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("Invalid timezone") || err.contains("invalid timezone"),
        "Error should mention invalid timezone, got: {}",
        err
    );
}
// =============================================================================
// Scenario: Reject agent job missing required fields
// =============================================================================

#[tokio::test]
async fn test_reject_agent_job_missing_required_fields() {
    // @step Given a session with a registered schedule handler
    // @step And an empty schedules.json file
    let tmp = setup_empty_project().await;
    let handler = create_handler(tmp.path().to_string_lossy().to_string());

    // @step When the Schedule tool is called with action "add", name "test", cron "0 9 * * *", timezone "UTC", job_type "agent", without role or prompt
    let req = ScheduleRequest {
        action: "add".to_string(),
        name: Some("test".to_string()),
        cron: Some("0 9 * * *".to_string()),
        timezone: Some("UTC".to_string()),
        job_type: Some("agent".to_string()),
        role: None,
        prompt: None,
        command: None,
        overlap_policy: None,
    };
    let result = handler(req);

    // @step Then the response should have success false
    assert!(!result.success, "Expected failure for missing agent fields");

    // @step And the error message should contain "Agent jobs require"
    let err = result.error.expect("Expected error message");
    assert!(
        err.contains("Agent jobs require") || err.contains("agent jobs require"),
        "Error should mention agent requirements, got: {}",
        err
    );
}
