//! AgentManager Tool — Spawn subordinate agents, manage lifecycle, messaging, and coordination
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-messaging.feature
//! Feature: spec/features/agent-manager-context-resolution.feature
//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! This tool provides seven core actions for agent lifecycle, messaging, and coordination:
//! - `spawn`: Create a new subordinate session with optional role
//! - `list`: List all sessions with their relationships
//! - `get_status`: Get detailed status of a specific session
//! - `close`: Terminate a subordinate session (spawner-only)
//! - `message`: Send a plain text message to any session by ID, with optional context references
//! - `set_role`: Set or replace the system prompt overlay on a session
//! - `await_idle`: Block until one or more sessions become idle (AMGR-015)
//!
//! The tool uses the handler pattern (like SessionSearchTool) to delegate to the
//! session management layer in codelet-napi. The tool definition and schema live here,
//! the actual session operations are registered via `set_agent_manager_handler()`.
//! For async actions (await_idle), a separate async handler is used via
//! `set_agent_manager_async_handler()`.

pub mod handler;
pub mod types;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

use rig::tool::Tool;
use serde_json::json;

use crate::ToolError;
use handler::{execute_agent_manager, execute_agent_manager_async};
use types::AgentManagerArgs;
use uuid::Uuid;

pub use handler::{
    clear_all_agent_manager_handlers, has_agent_manager_handler,
    set_agent_manager_handler, set_agent_manager_async_handler,
    AgentManagerHandler, AgentManagerAsyncHandler,
};
pub use types::{
    AgentManagerAction, AgentManagerArgs as Args, AgentManagerResult,
    AwaitOutcome, AwaitSessionResult, SessionEntry, SessionIdParam,
    SessionStatus,
};

/// AgentManager Tool — Rig Tool implementation
///
/// Allows AI agents to spawn and manage subordinate sessions.
/// Uses handler mechanism for session-scoped access to SessionManager.
#[derive(Clone, Debug)]
pub struct AgentManagerTool {
    session_id: Uuid,
}

impl AgentManagerTool {
    /// Create a new AgentManagerTool instance
    ///
    /// # Arguments
    /// * `session_id` - The calling session's ID for handler lookup and permission checks
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for AgentManagerTool {
    const NAME: &'static str = "AgentManager";

    type Error = ToolError;
    type Args = AgentManagerArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "AgentManager".to_string(),
            description: concat!(
                "Manage subordinate AI agent sessions. ",
                "Actions: 'spawn' (create a new subordinate session with optional role), ",
                "'list' (show all sessions with relationships), ",
                "'get_status' (detailed info for a specific session), ",
                "'close' (terminate a subordinate session — spawner only), ",
                "'message' (send a message to any session by ID, with optional context references), ",
                "'set_role' (set or replace the system prompt overlay on a session), ",
                "'await_idle' (block until one or more sessions become idle — use instead of polling get_status with sleep). ",
                "Spawned sessions inherit the spawner's model and start idle. ",
                "Use 'spawn' to create workers, send them tasks via 'message', ",
                "monitor with 'list'/'get_status', and clean up with 'close'. ",
                "Context references on 'message' allow quoting session history: ",
                "specific turns, turn ranges, or search queries resolved at send time."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["spawn", "list", "get_status", "close", "message", "set_role", "await_idle"],
                        "description": "The action to perform"
                    },
                    "role": {
                        "type": ["string", "null"],
                        "description": "Role string — system prompt overlay. For 'spawn': optional role for subordinate. For 'set_role': the role text to set (empty string clears)."
                    },
                    "session_id": {
                        "oneOf": [
                            { "type": "string" },
                            { "type": "array", "items": { "type": "string" } }
                        ],
                        "description": "Target session ID (required for get_status, close, message; optional for set_role — defaults to caller's own session). For await_idle: one or more session IDs to wait for."
                    },
                    "message": {
                        "type": ["string", "null"],
                        "description": "Message text to send to the target session (required for message action)"
                    },
                    "context": {
                        "type": ["array", "null"],
                        "description": "Optional context references to include with the message. Each element references session history: {session_id, turns: [0,1]} for specific turns, {session_id, start_turn: 0, end_turn: 5} for a range, or {session_id, query: 'search term'} for a search. Resolved at send time.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "session_id": {
                                    "type": "string",
                                    "description": "Session ID whose history to reference"
                                },
                                "turns": {
                                    "type": "array",
                                    "items": { "type": "integer" },
                                    "description": "Specific turn indices to include (0-based)"
                                },
                                "start_turn": {
                                    "type": "integer",
                                    "description": "Start of turn range (inclusive, 0-based)"
                                },
                                "end_turn": {
                                    "type": "integer",
                                    "description": "End of turn range (inclusive)"
                                },
                                "query": {
                                    "type": "string",
                                    "description": "Search query (ripgrep regex) to find matching turns"
                                }
                            },
                            "required": ["session_id"]
                        }
                    },
                    "timeout": {
                        "type": ["integer", "null"],
                        "description": "Optional maximum wait time in seconds for await_idle. If omitted, waits indefinitely until all sessions are idle.",
                        "minimum": 0
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
            &serde_json::to_value(&args.action).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "agent_manager",
                message: reason,
            });
        }

        // AMGR-015: Route await_idle to the async handler, all others to sync
        let result = match &args.action {
            AgentManagerAction::AwaitIdle { .. } => {
                execute_agent_manager_async(self.session_id, args.action).await
            }
            _ => execute_agent_manager(self.session_id, args.action),
        };

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "agent_manager",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}
