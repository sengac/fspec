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

use codelet_agent_loop::schedule_handler::create_handler;
use codelet_tools::schedule::handler::{
    clear_all_schedule_handlers, execute_schedule_command, has_schedule_handler,
};
use codelet_tools::schedule::types::ScheduleRequest;

#[path = "schedule_tool_test/provider_registration.rs"]
mod provider_registration;
#[path = "schedule_tool_test/provider_source.rs"]
mod provider_source;
#[path = "schedule_tool_test/request_validation.rs"]
mod request_validation;
#[path = "schedule_tool_test/schedule_actions.rs"]
mod schedule_actions;

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
fn add_agent_request(
    name: &str,
    cron: &str,
    tz: &str,
    role: &str,
    prompt: &str,
) -> ScheduleRequest {
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
