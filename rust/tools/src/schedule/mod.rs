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

    /// TOOL-024: classify a schedule handler error message. Missing or
    /// invalid parameters (required-field failures, invalid
    /// cron/timezone/job_type values, unknown actions) are
    /// argument-validation failures; operational failures (schedule not
    /// found, file I/O, no handler registered, duplicate name) are
    /// execution failures.
    fn is_arg_validation_error(message: &str) -> bool {
        message.contains("is required")
            || message.contains("Invalid cron expression")
            || message.contains("Invalid timezone")
            || message.contains("Invalid job_type")
            || message.contains("jobs require")
            || message.starts_with("Unknown action")
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
                        "default": "skip",
                        "description": "Overlap policy (optional for add, default: skip)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-013: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            &self.name(),
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Schedule",
                message: reason,
            });
        }

        let request = ScheduleRequest {
            action: args.action.clone(),
            name: args.name.clone(),
            cron: args.cron.clone(),
            timezone: args.timezone.clone(),
            job_type: args.job_type.clone(),
            role: args.role.clone(),
            prompt: args.prompt.clone(),
            command: args.command.clone(),
            overlap_policy: args.overlap_policy.clone(),
        };

        let result = execute_schedule_command(self.session_id, request);

        // TOOL-024: a failed ScheduleResult must surface as a proper tool
        // error, not a serialized JSON blob. Argument-validation failures
        // (missing/invalid parameters on the requested action) get the
        // full recovery surface; other failures (unknown schedule, file
        // I/O, no handler registered) stay execution errors.
        if !result.success {
            let error = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown schedule error".to_string());
            if Self::is_arg_validation_error(&error) {
                let schema = self.definition(String::new()).await.parameters;
                // State the action context: the handler message (e.g.
                // "Cron expression is required") is preserved verbatim as
                // a substring, and the rewording makes the
                // conditionally-required parameter explicit ("cron is
                // required for the add action").
                let message = format!("{} action: {error}", args.action);
                // The synthesized example covers only schema-`required`
                // fields, but add's parameters are conditionally required —
                // supply the canonical add call explicitly.
                let example = r#"{"action": "add", "name": "<name>", "cron": "0 2 * * *", "timezone": "UTC", "job_type": "agent", "role": "<role>", "prompt": "<prompt>", "overlap_policy": "skip"}"#;
                return Err(ToolError::Validation {
                    tool: "Schedule",
                    message: codelet_common::tool_usage::append_usage_to_message_with_example(
                        "Schedule", &schema, &message, example,
                    ),
                });
            }
            return Err(ToolError::Execution {
                tool: "Schedule",
                message: error,
            });
        }

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "Schedule",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}
