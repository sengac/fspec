//! Schedule tool types — request/response structures for the handler-delegated pattern
//!
//! Feature: spec/features/schedule-ai-tool.feature
//!
//! These types are shared between the tool definition (codelet-tools),
//! the handler registry (codelet-tools), and the handler implementation (codelet-napi).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request from the Schedule tool to the registered handler.
///
/// Maps 1:1 to ScheduleArgs but is the handler-internal type (not schema-bound).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    /// Action: add, list, pause, resume, remove
    pub action: String,
    /// Schedule name (required for add, pause, resume, remove)
    pub name: Option<String>,
    /// Cron expression (required for add)
    pub cron: Option<String>,
    /// IANA timezone (required for add)
    pub timezone: Option<String>,
    /// Job type: "agent" or "shell" (required for add)
    pub job_type: Option<String>,
    /// Agent role (required for add with job_type=agent)
    pub role: Option<String>,
    /// Agent prompt (required for add with job_type=agent)
    pub prompt: Option<String>,
    /// Shell command (required for add with job_type=shell)
    pub command: Option<String>,
    /// Overlap policy: "skip" or "queue" (optional for add, default: skip)
    pub overlap_policy: Option<String>,
}

/// Result returned by the schedule handler to the tool.
///
/// Serialized to JSON and returned to the LLM as the tool output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The action that was performed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Single schedule data (for add, pause, resume)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<Value>,
    /// List of schedules (for list action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedules: Option<Vec<Value>>,
    /// Schedule name (for remove action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Error message if success is false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ScheduleResult {
    /// Create a success result for a single schedule operation
    pub fn success_schedule(action: &str, schedule: Value) -> Self {
        Self {
            success: true,
            action: Some(action.to_string()),
            schedule: Some(schedule),
            schedules: None,
            name: None,
            error: None,
        }
    }

    /// Create a success result for the list operation
    pub fn success_list(schedules: Vec<Value>) -> Self {
        Self {
            success: true,
            action: Some("list".to_string()),
            schedule: None,
            schedules: Some(schedules),
            name: None,
            error: None,
        }
    }

    /// Create a success result for the remove operation
    pub fn success_remove(name: &str) -> Self {
        Self {
            success: true,
            action: Some("remove".to_string()),
            schedule: None,
            schedules: None,
            name: Some(name.to_string()),
            error: None,
        }
    }

    /// Create an error result
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            action: None,
            schedule: None,
            schedules: None,
            name: None,
            error: Some(message.to_string()),
        }
    }
}
