//! Scheduler types — Rust-side schedule data structures
//!
//! These mirror the TypeScript types from src/types/schedule.ts (SCHED-002).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level schedules.json file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesFile {
    pub version: String,
    pub schedules: HashMap<String, ScheduleEntry>,
}

/// A single schedule entry (agent or shell)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub cron: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub job_type: String,
    pub created_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_status: Option<String>,
    /// Agent-specific config
    pub agent: Option<AgentConfig>,
    /// Shell-specific config
    pub shell: Option<ShellConfig>,
    /// Overlap policy (delegated to SCHED-006)
    pub overlap_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub command: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_status() -> String {
    "active".to_string()
}

/// Result of evaluating a single schedule
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub name: String,
    pub triggered: bool,
    pub job_type: String,
    pub evaluated_timezone: String,
    pub error: Option<String>,
    /// The full schedule entry, carried forward for trigger_and_update
    pub entry: ScheduleEntry,
}
