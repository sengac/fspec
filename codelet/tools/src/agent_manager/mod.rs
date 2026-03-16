//! AgentManager Tool — Spawn subordinate agents, manage lifecycle, messaging
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-messaging.feature
//! Feature: spec/features/agent-manager-context-resolution.feature
//!
//! This tool provides five core actions for agent lifecycle and messaging:
//! - `spawn`: Create a new subordinate session with optional role
//! - `list`: List all sessions with their relationships
//! - `get_status`: Get detailed status of a specific session
//! - `close`: Terminate a subordinate session (spawner-only)
//! - `message`: Send a plain text message to any session by ID
//!
//! The tool uses the handler pattern (like SessionSearchTool) to delegate to the
//! session management layer in codelet-napi. The tool definition and schema live here,
//! the actual session operations are registered via `set_agent_manager_handler()`.

pub mod handler;
pub mod types;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

use rig::tool::Tool;
use serde_json::json;

use crate::ToolError;
use handler::execute_agent_manager;
use types::AgentManagerArgs;
use uuid::Uuid;

pub use handler::{
    clear_all_agent_manager_handlers, has_agent_manager_handler,
    set_agent_manager_handler, AgentManagerHandler,
};
pub use types::{
    AgentManagerAction, AgentManagerArgs as Args, AgentManagerResult,
    SessionEntry, SessionStatus,
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
                "'set_role' (set or replace the system prompt overlay on a session). ",
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
                        "enum": ["spawn", "list", "get_status", "close", "message", "set_role"],
                        "description": "The action to perform"
                    },
                    "role": {
                        "type": ["string", "null"],
                        "description": "Role string — system prompt overlay. For 'spawn': optional role for subordinate. For 'set_role': the role text to set (empty string clears)."
                    },
                    "session_id": {
                        "type": ["string", "null"],
                        "description": "Target session ID (required for get_status, close, message; optional for set_role — defaults to caller's own session)"
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
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = execute_agent_manager(self.session_id, args.action);

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "agent_manager",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}
