//! Schedule AI Tool — manages scheduled jobs programmatically
//!
//! Feature: spec/features/schedule-ai-tool.feature
//!
//! Allows AI agents to add, list, pause, resume, and remove scheduled jobs.
//! Follows the handler-delegated pattern: tool definition here, handler registry
//! here, handler implementation in codelet-napi.

pub mod handler;
pub mod types;

use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::ToolError;
use handler::execute_schedule_command;
use types::ScheduleRequest;

pub use handler::{
    clear_all_schedule_handlers, has_schedule_handler, set_schedule_handler, ScheduleHandler,
};
pub use types::ScheduleResult;

/// Arguments for the Schedule tool (deserialized from LLM JSON).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ScheduleArgs {
    /// Action: add, list, pause, resume, remove
    pub action: String,
    /// Schedule name (required for add, pause, resume, remove)
    #[serde(default)]
    pub name: Option<String>,
    /// Cron expression (required for add)
    #[serde(default)]
    pub cron: Option<String>,
    /// IANA timezone (required for add)
    #[serde(default)]
    pub timezone: Option<String>,
    /// Job type: "agent" or "shell" (required for add)
    #[serde(default)]
    pub job_type: Option<String>,
    /// Agent role (required for add with job_type=agent)
    #[serde(default)]
    pub role: Option<String>,
    /// Agent prompt (required for add with job_type=agent)
    #[serde(default)]
    pub prompt: Option<String>,
    /// Shell command (required for add with job_type=shell)
    #[serde(default)]
    pub command: Option<String>,
    /// Overlap policy: "skip" or "queue" (optional for add, default: skip)
    #[serde(default)]
    pub overlap_policy: Option<String>,
}

/// Schedule AI Tool — Rig Tool implementation
///
/// Constructed per-session with the session's UUID.
/// Delegates to the registered handler via execute_schedule_command().
#[derive(Clone, Debug)]
pub struct ScheduleTool {
    session_id: Uuid,
}

impl ScheduleTool {
    /// Create a new ScheduleTool instance
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for ScheduleTool {
    const NAME: &'static str = "Schedule";

    type Error = ToolError;
    type Args = ScheduleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Schedule".to_string(),
            description: concat!(
                "Manage scheduled jobs. Actions: ",
                "'add' (create a new schedule with cron, timezone, and job config), ",
                "'list' (show all schedules with status and next run), ",
                "'pause' (suspend a schedule), ",
                "'resume' (reactivate a paused schedule), ",
                "'remove' (delete a schedule). ",
                "Supports agent jobs (role+prompt) and shell jobs (command)."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["add", "list", "pause", "resume", "remove"],
                        "description": "The action to perform"
                    },
                    "name": {
                        "type": ["string", "null"],
                        "description": "Schedule name (required for add, pause, resume, remove)"
                    },
                    "cron": {
                        "type": ["string", "null"],
                        "description": "Cron expression, e.g. '0 2 * * *' (required for add)"
                    },
                    "timezone": {
                        "type": ["string", "null"],
                        "description": "IANA timezone, e.g. 'Australia/Sydney' (required for add)"
                    },
                    "job_type": {
                        "type": ["string", "null"],
                        "enum": ["agent", "shell"],
                        "description": "Job type (required for add)"
                    },
                    "role": {
                        "type": ["string", "null"],
                        "description": "Agent role (required for add with job_type=agent)"
                    },
                    "prompt": {
                        "type": ["string", "null"],
                        "description": "Agent prompt (required for add with job_type=agent)"
                    },
                    "command": {
                        "type": ["string", "null"],
                        "description": "Shell command (required for add with job_type=shell)"
                    },
                    "overlap_policy": {
                        "type": ["string", "null"],
                        "enum": ["skip", "queue"],
                        "description": "Overlap policy (optional for add, default: skip)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let request = ScheduleRequest {
            action: args.action,
            name: args.name,
            cron: args.cron,
            timezone: args.timezone,
            job_type: args.job_type,
            role: args.role,
            prompt: args.prompt,
            command: args.command,
            overlap_policy: args.overlap_policy,
        };

        let result = execute_schedule_command(self.session_id, request);

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "schedule",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}
