//! Feature: spec/features/shell-job-execution.feature
//!
//! Tests for Shell Job Execution (SCHED-005).
//! Validates shell command execution when schedules fire,
//! including stdout/stderr capture, exit code handling, config validation,
//! and schedules.json timestamp updates.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

#[path = "shell_job_test/command_execution.rs"]
mod command_execution;
#[path = "shell_job_test/config_and_engine.rs"]
mod config_and_engine;

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

/// Helper: read schedules.json from a project directory
async fn read_schedules(project_path: &Path) -> serde_json::Value {
    let path = project_path.join("spec/schedules.json");
    let content = fs::read_to_string(&path).await.expect("read schedules");
    serde_json::from_str(&content).expect("parse schedules")
}

/// Helper: build a shell schedule entry
fn make_shell_schedule(command: &str) -> serde_json::Value {
    json!({
        "cron": "*/5 * * * *",
        "timezone": "UTC",
        "status": "active",
        "job_type": "shell",
        "shell": {
            "command": command
        },
        "created_at": "2026-03-18T00:00:00Z"
    })
}

/// Helper: build a shell schedule entry with no shell config
fn make_shell_schedule_no_config() -> serde_json::Value {
    json!({
        "cron": "*/5 * * * *",
        "timezone": "UTC",
        "status": "active",
        "job_type": "shell",
        "created_at": "2026-03-18T00:00:00Z"
    })
}
