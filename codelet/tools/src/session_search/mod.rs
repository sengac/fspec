//! SessionSearch Tool — Native Rust session history search
//!
//! Feature: spec/features/session-search.feature
//!
//! This tool provides three actions for searching and viewing session history:
//! - `recent`: List recent sessions for discovery
//! - `search`: Keyword search across all session content with context
//! - `show`: Load and display a specific session's conversation
//!
//! Replaces scripts/session-search.sh and scripts/session-search-skill.md.
//!
//! The tool uses the handler pattern (like FspecTool) to delegate to the
//! persistence layer in codelet-napi. The tool definition and schema live here,
//! the actual data access is registered via `set_session_search_handler()`.

pub mod handler;
pub mod reassembly;
pub mod types;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod tests;

use rig::tool::Tool;
use serde_json::json;

use crate::ToolError;
use handler::execute_session_search;
use types::SessionSearchArgs;
use uuid::Uuid;

pub use handler::{
    clear_all_session_search_handlers, has_session_search_handler, set_session_search_handler,
    SessionSearchHandler,
};
pub use types::{
    ContextTurn, SearchMatch, SearchMatchGroup, SessionMessage,
    SessionSearchResult as SearchResult, SessionSummary, DEFAULT_RECENT_COUNT,
    DEFAULT_SEARCH_LIMIT, MESSAGE_TRUNCATION_LIMIT, USER_MESSAGE_PREVIEW_LEN,
};

/// SessionSearch Tool — Rig Tool implementation
///
/// Allows AI agents to search and view session conversation history.
/// Uses handler mechanism for session-scoped persistence access.
#[derive(Clone, Debug)]
pub struct SessionSearchTool {
    session_id: Uuid,
}

impl SessionSearchTool {
    /// Create a new SessionSearchTool instance
    ///
    /// # Arguments
    /// * `session_id` - The session ID for "current session" resolution
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for SessionSearchTool {
    const NAME: &'static str = "SessionSearch";

    type Error = ToolError;
    type Args = SessionSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "SessionSearch".to_string(),
            description: concat!(
                "Search and view session conversation history. ",
                "Three actions: 'recent' (list recent sessions), ",
                "'search' (keyword search with regex across all content — user inputs, ",
                "assistant responses, tool calls), ",
                "'show' (load specific session or current session). ",
                "Use recent → search → show to iteratively find relevant context. ",
                "Results include session names, work unit IDs, timestamps, and content previews."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "action_type": {
                        "type": "string",
                        "enum": ["recent", "search", "show"],
                        "description": "The action to perform"
                    },
                    "count": {
                        "type": ["integer", "null"],
                        "description": "Number of recent sessions to return (recent action, default: 10)"
                    },
                    "query": {
                        "type": ["string", "null"],
                        "description": "Search query — supports ripgrep regex (search action, required)"
                    },
                    "context_turns": {
                        "type": ["integer", "null"],
                        "description": "Number of surrounding turns to include per match (search action, default: 0)"
                    },
                    "limit": {
                        "type": ["integer", "null"],
                        "description": "Maximum matches to return (search action, default: 20)"
                    },
                    "all_projects": {
                        "type": ["boolean", "null"],
                        "description": "Search all projects, not just current (search action, default: false)"
                    },
                    "last_hours": {
                        "type": ["integer", "null"],
                        "description": "Only search sessions updated in last N hours (search action)"
                    },
                    "last_days": {
                        "type": ["integer", "null"],
                        "description": "Only search sessions updated in last N days (search action)"
                    },
                    "after": {
                        "type": ["string", "null"],
                        "description": "Only search sessions updated after ISO timestamp (search action)"
                    },
                    "before": {
                        "type": ["string", "null"],
                        "description": "Only search sessions updated before ISO timestamp (search action)"
                    },
                    "session_id": {
                        "type": ["string", "null"],
                        "description": "Session ID to show — UUID or 'current' (show action, default: current)"
                    },
                    "user_only": {
                        "type": ["boolean", "null"],
                        "description": "Only include user messages (show action, default: false)"
                    },
                    "max_turns": {
                        "type": ["integer", "null"],
                        "description": "Maximum turns to include from the end (show action)"
                    },
                    "start_turn": {
                        "type": ["integer", "null"],
                        "description": "Start of turn range (inclusive, 0-based) to restrict results (optional for 'show' and 'search' actions)"
                    },
                    "end_turn": {
                        "type": ["integer", "null"],
                        "description": "End of turn range (inclusive, 0-based) to restrict results (optional for 'show' and 'search' actions)"
                    }
                },
                "required": ["action_type"]
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
                tool: "session_search",
                message: reason,
            });
        }

        let result = execute_session_search(self.session_id, args.action);

        serde_json::to_string_pretty(&result).map_err(|e| ToolError::Execution {
            tool: "session_search",
            message: format!("Failed to serialize result: {e}"),
        })
    }
}
