#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
//! Feature: spec/features/overlap-session-limit.feature
//!
//! Tests for Overlap & Session Limit Management (SCHED-006).
//! Validates skip/queue overlap policies, session limit deferral,
//! completion detection, and queue drain behavior.

use codelet_core::scheduler::types::ScheduleEntry;
use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;
use uuid::Uuid;

#[path = "overlap_session_limit_test/overlap_policy.rs"]
mod overlap_policy;
#[path = "overlap_session_limit_test/session_limit.rs"]
mod session_limit;

// =====================================================================
// SchedulerState — the core struct under test
// We test the state module's logic directly.
// =====================================================================

/// Mirrors the SchedulerState that will be created in the implementation.
/// We test behavior against this expected interface.
///
/// Helper: build an agent schedule entry with an overlap policy
fn make_agent_schedule(overlap_policy: Option<&str>) -> ScheduleEntry {
    ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "agent".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: Some(codelet_core::scheduler::types::AgentConfig {
            role: Some("test".to_string()),
            prompt: Some("test prompt".to_string()),
        }),
        shell: None,
        overlap_policy: overlap_policy.map(std::string::ToString::to_string),
    }
}

/// Helper: build a shell schedule entry
fn make_shell_schedule() -> ScheduleEntry {
    ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "echo test".to_string(),
        }),
        overlap_policy: None,
    }
}
