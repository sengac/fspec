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
//! - `await_idle`: Block until one or more sessions become idle (AMGR-015),
//!   or one of them needs the caller's answer (AMGR-021)
//! - `answer` / `approve` / `escalate`: reply to, or pass up, a sub-agent's
//!   question or approval the caller holds (AMGR-021)
//!
//! The tool uses the handler pattern (like SessionSearchTool) to delegate to the
//! session management layer in codelet-napi. The tool definition and schema live here,
//! the actual session operations are registered via `set_agent_manager_handler()`.
//! For async actions (await_idle), a separate async handler is used via
//! `set_agent_manager_async_handler()`.

pub mod handler;
pub mod types;

// AgentManager tests: one file per feature (COV-075); shared fixtures in
// test_support.rs. Each file allows unwrap/expect/panic/needless_collect.
#[cfg(test)]
mod await_idle_cancel_tests;
#[cfg(test)]
mod await_idle_tests;
#[cfg(test)]
mod await_idle_types_tests;
#[cfg(test)]
mod context_mixed_tests;
#[cfg(test)]
mod context_search_tests;
#[cfg(test)]
mod context_serde_tests;
#[cfg(test)]
mod context_turns_tests;
#[cfg(test)]
mod core_close_tests;
#[cfg(test)]
mod core_tests;
#[cfg(test)]
mod core_types_tests;
#[cfg(test)]
mod messaging_dispatch_tests;
#[cfg(test)]
mod messaging_tests;
#[cfg(test)]
mod messaging_validation_tests;
#[cfg(test)]
mod set_role_tests;
#[cfg(test)]
mod test_support;

use rig::tool::Tool;
use serde_json::json;

use crate::ToolError;
use handler::{execute_agent_manager, execute_agent_manager_async};
use types::AgentManagerArgs;
use uuid::Uuid;

pub use handler::{
    clear_all_agent_manager_handlers, has_agent_manager_handler, set_agent_manager_async_handler,
    set_agent_manager_handler, AgentManagerAsyncHandler, AgentManagerHandler,
};
pub use types::{
    AgentManagerAction, AgentManagerArgs as Args, AgentManagerResult, ApprovalView, ApproveChoice,
    AwaitOutcome, AwaitSessionResult, PendingRequestView, RequestAnswer, SessionEntry,
    SessionIdParam, SessionStatus,
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
                "'message' (send a message to idle recipients by ID, with optional context references), ",
                "'set_role' (set or replace the system prompt overlay on a session), ",
                "'await_idle' (block until one or more sessions become idle — use instead of polling get_status with sleep; it returns at once with status needs_input when a sub-agent, or one below it, asks you a question or needs your approval, naming the request, and with waiting_on_user, once, when a sub-agent waits on the user), ",
                "'answer' (answer a question you hold: request_id plus answers [{id, selected, other}] or cancelled true), ",
                "'approve' (decide an approval you hold: request_id plus choice approve, approve_session or deny), ",
                "'escalate' (pass a request you hold up to your own parent, or to the user at the root), ",
                "'profile' (run a time-bounded runtime profiling window — BLOCKS for duration_secs seconds, default 10). ",
                "Spawned sessions inherit the spawner's model and start idle. ",
                "Root work_unit_id attributes the calling session, never the spawned or targeted session, for every action including spawn, message, get_status and profile. ",
                "For example, orchestrating AUTH-001 while delegating AUTH-002: use work_unit_id='AUTH-001' on your management calls and delegate AUTH-002 in role or message text. ",
                "This does not assign worker activity focus; workers must attribute their own calls to AUTH-002. ",
                "Use exact NONE to clear only your own activity, not to retain it or change permissions. ",
                "Messages to busy recipients return session_busy and are NOT queued; a sub-agent paused on a request waits for answer, approve or escalate, never a message. ",
                "Use 'await_idle', then send a fresh message once the recipient is idle. ",
                "If the recipient is waiting for you, finish your task and return results instead of waiting on it. ",
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
                        "enum": ["spawn", "list", "get_status", "close", "message", "set_role", "await_idle", "answer", "approve", "escalate", "profile"],
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
                    "request_id": {
                        "type": ["string", "null"],
                        "description": "For 'answer', 'approve' and 'escalate': the id of the request, from await_idle's needs_input result or the message that told you about it."
                    },
                    "answers": {
                        "type": ["array", "null"],
                        "description": "For 'answer': one entry per question: {id: the question id, selected: [option labels], other: free text}.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "string" },
                                "selected": { "type": "array", "items": { "type": "string" } },
                                "other": { "type": ["string", "null"] }
                            },
                            "required": ["id"]
                        }
                    },
                    "cancelled": {
                        "type": ["boolean", "null"],
                        "description": "For 'answer': true to cancel the question instead of answering it."
                    },
                    "choice": {
                        "type": ["string", "null"],
                        "enum": ["approve", "approve_session", "deny", null],
                        "description": "For 'approve': approve (this once), approve_session (for the asking sub-agent's session) or deny."
                    },
                    "timeout": {
                        "type": ["integer", "null"],
                        "description": "Optional maximum wait time in seconds for await_idle. If omitted, waits indefinitely until all sessions are idle.",
                        "minimum": 0
                    },
                    "duration_secs": {
                        "type": ["integer", "null"],
                        "description": "For 'profile' action: length of the profiling window in seconds (1..=60, default 10). The tool call BLOCKS for this entire duration — callers must expect the wait and not interpret it as a hang.",
                        "minimum": 1,
                        "maximum": 60
                    },
                    "top_n": {
                        "type": ["integer", "null"],
                        "description": "For 'profile' action: cap on the number of scopes returned in scopes_by_calls and scopes_by_self_ms (default 20, max 200).",
                        "minimum": 1,
                        "maximum": 200
                    },
                    "label_prefix": {
                        "type": ["string", "null"],
                        "description": "For 'profile' action: only scopes whose label starts with this string appear in the result."
                    },
                    "focus": {
                        "type": ["string", "null"],
                        "description": "For 'profile' action (AMGR-018): optional substring filter applied BEFORE attribution. Any sample whose stack does not contain a frame whose symbol contains this substring is dropped, so hot_stacks, scopes_by_calls, scopes_by_self_ms and samples_by_thread all reflect the narrowed view. Use this to confirm or rule out a suspected hot function in one call."
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

        // AMGR-015/AMGR-017: Route await_idle and profile to the async handler, all others to sync
        let result = match &args.action {
            AgentManagerAction::AwaitIdle { .. } | AgentManagerAction::Profile { .. } => {
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
