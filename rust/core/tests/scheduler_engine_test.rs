//! Feature: spec/features/core-scheduler-engine.feature
//!
//! Tests for the Core Scheduler Engine (SCHED-003).
//! Validates cron evaluation, timezone handling, error resilience,
//! and job delegation for the scheduler loop.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs;

#[path = "scheduler_engine_test/engine_runs.rs"]
mod engine_runs;
#[path = "scheduler_engine_test/trigger_evaluation.rs"]
mod trigger_evaluation;

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
