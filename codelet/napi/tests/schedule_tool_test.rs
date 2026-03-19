#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/schedule-ai-tool.feature
//!
//! This test file validates the acceptance criteria for the Schedule AI Tool
//! (SCHED-009). Tests exercise the handler implementation directly — creating
//! a handler via create_handler() and calling it with ScheduleRequest values.
//!
//! Scenarios map directly to Gherkin scenarios:
//! 1. Add an agent-type schedule
//! 2. Add a shell-type schedule
//! 3. List all schedules
//! 4. Pause an active schedule
//! 5. Resume a paused schedule
//! 6. Remove an existing schedule
//! 7. Reject invalid cron expression
//! 8. Reject duplicate schedule name
//! 9. Reject removal of nonexistent schedule
//! 10. Graceful error when no handler is registered
//! 11. Reject invalid timezone
//! 12. Reject agent job missing required fields

use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

use codelet_napi::schedule_handler::create_handler;
use codelet_tools::schedule::handler::{
    clear_all_schedule_handlers, execute_schedule_command, has_schedule_handler,
};
use codelet_tools::schedule::types::ScheduleRequest;

// =============================================================================
// Test Helpers
// =============================================================================

/// Helper: create a temp project directory with an empty schedules.json
async fn setup_empty_project() -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let spec_dir = tmp.path().join("spec");
    fs::create_dir_all(&spec_dir)
        .await
        .expect("create spec dir");
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {}
    });
    fs::write(
        spec_dir.join("schedules.json"),
        serde_json::to_string_pretty(&schedules).unwrap(),
    )
    .await
    .expect("write schedules.json");
    tmp
}

/// Helper: create a temp project directory with pre-existing schedules
async fn setup_project_with_schedules(schedules: serde_json::Value) -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let spec_dir = tmp.path().join("spec");
    fs::create_dir_all(&spec_dir)
        .await
        .expect("create spec dir");
    let data = json!({
        "version": "1.0.0",
        "schedules": schedules
    });
    fs::write(
        spec_dir.join("schedules.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .await
    .expect("write schedules.json");
    tmp
}

/// Helper: read schedules.json and return the schedules object
async fn read_schedules(project_path: &Path) -> serde_json::Value {
    let path = project_path.join("spec/schedules.json");
    let content = fs::read_to_string(&path).await.expect("read schedules");
    let data: serde_json::Value = serde_json::from_str(&content).expect("parse schedules");
    data["schedules"].clone()
}

/// Helper: create a request for adding an agent schedule
fn add_agent_request(name: &str, cron: &str, tz: &str, role: &str, prompt: &str) -> ScheduleRequest {
    ScheduleRequest {
        action: "add".to_string(),
        name: Some(name.to_string()),
        cron: Some(cron.to_string()),
        timezone: Some(tz.to_string()),
        job_type: Some("agent".to_string()),
        role: Some(role.to_string()),
        prompt: Some(prompt.to_string()),
        command: None,
        overlap_policy: None,
    }
}

/// Helper: create a request for adding a shell schedule
fn add_shell_request(name: &str, cron: &str, tz: &str, cmd: &str) -> ScheduleRequest {
    ScheduleRequest {
        action: "add".to_string(),
        name: Some(name.to_string()),
        cron: Some(cron.to_string()),
        timezone: Some(tz.to_string()),
        job_type: Some("shell".to_string()),
        role: None,
        prompt: None,
        command: Some(cmd.to_string()),
        overlap_policy: None,
    }
}

/// Helper: create a simple action request (pause, resume, remove)
fn action_request(action: &str, name: &str) -> ScheduleRequest {
    ScheduleRequest {
        action: action.to_string(),
        name: Some(name.to_string()),
        cron: None,
        timezone: None,
        job_type: None,
        role: None,
        prompt: None,
        command: None,
        overlap_policy: None,
    }
}

/// Helper: create a list request
fn list_request() -> ScheduleRequest {
    ScheduleRequest {
        action: "list".to_string(),
        name: None,
        cron: None,
        timezone: None,
        job_type: None,
        role: None,
        prompt: None,
        command: None,
        overlap_policy: None,
    }
}

/// Helper: build an agent schedule entry for pre-populating schedules
fn agent_entry(cron: &str, tz: &str, status: &str) -> serde_json::Value {
    json!({
        "cron": cron,
        "timezone": tz,
        "status": status,
        "job_type": "agent",
        "created_at": "2026-01-01T00:00:00Z",
        "agent": {
            "role": "test agent",
            "prompt": "do something"
        }
    })
}

/// Helper: build a shell schedule entry for pre-populating schedules
fn shell_entry(cron: &str, tz: &str, status: &str) -> serde_json::Value {
    json!({
        "cron": cron,
        "timezone": tz,
        "status": status,
        "job_type": "shell",
        "created_at": "2026-01-01T00:00:00Z",
        "shell": {
            "command": "echo hello"
        }
    })
}

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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("add"));

    // @step And the response schedule should have name "nightly-review", cron "0 2 * * *", timezone "Australia/Sydney", and job_type "agent"
    let schedule = result.schedule.expect("Expected schedule in response");
    assert_eq!(schedule["name"], "nightly-review");
    assert_eq!(schedule["cron"], "0 2 * * *");
    assert_eq!(schedule["timezone"], "Australia/Sydney");
    assert_eq!(schedule["jobType"].as_str().or(schedule["job_type"].as_str()), Some("agent"));

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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("add"));

    // @step And the response schedule should have name "daily-lint" and job_type "shell"
    let schedule = result.schedule.expect("Expected schedule in response");
    assert_eq!(schedule["name"], "daily-lint");
    assert_eq!(schedule["jobType"].as_str().or(schedule["job_type"].as_str()), Some("shell"));

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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("list"));

    // @step And the response should contain 2 schedules with names "nightly-review" and "daily-sync"
    let schedules = result.schedules.expect("Expected schedules list in response");
    assert_eq!(schedules.len(), 2, "Expected 2 schedules, got {}", schedules.len());

    let names: Vec<String> = schedules
        .iter()
        .filter_map(|s| s["name"].as_str().map(String::from))
        .collect();
    assert!(names.contains(&"nightly-review".to_string()), "Missing nightly-review");
    assert!(names.contains(&"daily-sync".to_string()), "Missing daily-sync");
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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("pause"));

    // @step And schedules.json should show "nightly-review" with status "paused"
    let schedules = read_schedules(tmp.path()).await;
    assert_eq!(
        schedules["nightly-review"]["status"],
        "paused",
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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("resume"));

    // @step And schedules.json should show "nightly-review" with status "active"
    let schedules = read_schedules(tmp.path()).await;
    assert_eq!(
        schedules["nightly-review"]["status"],
        "active",
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
    assert!(result.success, "Expected success, got error: {:?}", result.error);
    assert_eq!(result.action.as_deref(), Some("remove"));

    // @step And schedules.json should not contain a schedule named "nightly-review"
    let schedules = read_schedules(tmp.path()).await;
    assert!(
        schedules.get("nightly-review").is_none(),
        "Schedule 'nightly-review' should have been removed"
    );
}

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
    assert!(!result.success, "Expected failure when no handler registered");

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

// =============================================================================
// Scenario: ScheduleTool registered in all provider agent builders
// =============================================================================

/// Read a provider source file and return its contents
fn read_provider_source(filename: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Navigate from codelet/napi to codelet/providers/src
    let path = Path::new(manifest_dir)
        .parent()
        .expect("codelet dir")
        .join("providers")
        .join("src")
        .join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "Failed to read provider source file {}: {}",
            path.display(),
            e
        )
    })
}

#[test]
fn test_schedule_tool_registered_in_claude_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("claude.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "claude.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "claude.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}

#[test]
fn test_schedule_tool_registered_in_gemini_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("gemini.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "gemini.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "gemini.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}

#[test]
fn test_schedule_tool_registered_in_openai_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("openai.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "openai.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "openai.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}

#[test]
fn test_schedule_tool_registered_in_zai_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("zai.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "zai.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "zai.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}

#[test]
fn test_schedule_tool_registered_in_codex_provider() {
    // @step Given each provider's create_rig_agent method is called with a session_id
    let source = read_provider_source("codex/mod.rs");

    // @step When the agent is built with the tool chain
    assert!(
        source.contains("ScheduleTool"),
        "codex/mod.rs must import ScheduleTool from codelet_tools"
    );

    // @step Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "codex/mod.rs must register ScheduleTool::new(session_id) in create_rig_agent"
    );
}
