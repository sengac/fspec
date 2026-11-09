//! Feature: spec/features/agent-job-execution.feature
//!
//! Tests for Agent Job Execution (SCHED-004).
//! Validates spawning of agent sessions when schedules fire,
//! including role/prompt handling, error cases, schedule metadata,
//! and the schedule metadata the TUI reads to identify scheduled sessions.
//!
//! Exercises `codelet_core::scheduler::agent_job` with
//! `NoopSchedulerHooks` (no SessionManager is linked).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

#[path = "agent_job_test/config_validation.rs"]
mod config_validation;
#[path = "agent_job_test/session_spawn.rs"]
mod session_spawn;

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

/// Helper: build a standard agent schedule entry
fn make_agent_schedule(role: Option<&str>, prompt: Option<&str>) -> serde_json::Value {
    let mut agent = serde_json::Map::new();
    if let Some(r) = role {
        agent.insert("role".to_string(), json!(r));
    }
    if let Some(p) = prompt {
        agent.insert("prompt".to_string(), json!(p));
    }

    json!({
        "cron": "*/5 * * * *",
        "timezone": "UTC",
        "status": "active",
        "job_type": "agent",
        "agent": agent,
        "created_at": "2026-03-18T00:00:00Z"
    })
}

/// Helper: build an agent schedule entry with no agent config at all
fn make_agent_schedule_no_config() -> serde_json::Value {
    json!({
        "cron": "*/5 * * * *",
        "timezone": "UTC",
        "status": "active",
        "job_type": "agent",
        "created_at": "2026-03-18T00:00:00Z"
    })
}
