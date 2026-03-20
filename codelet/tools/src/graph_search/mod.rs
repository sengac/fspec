//! GraphSearch Tool — Definition & Entry Point
//!
//! Provides the `GraphSearchTool` struct that implements the Rig `Tool` trait.
//! Follows the same pattern as `SessionSearchTool`.

mod handler;
#[cfg(test)]
mod tests;
mod types;

pub use handler::{
    clear_all_graph_search_handlers, execute_graph_search, has_graph_search_handler,
    set_graph_search_handler, GraphSearchHandler,
};
pub use types::{GraphSearchAction, GraphSearchArgs};

use crate::ToolError;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use uuid::Uuid;

/// GraphSearch tool — queries the nanograph knowledge graph.
///
/// Each instance is bound to a session via `session_id`.
/// The actual query execution is delegated to a handler registered
/// by `codelet-napi` at session start.
pub struct GraphSearchTool {
    session_id: Uuid,
}

impl GraphSearchTool {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for GraphSearchTool {
    const NAME: &'static str = "GraphSearch";
    type Error = ToolError;
    type Args = GraphSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search and query the knowledge graph. Explore concepts, decisions, relationships, code entities, and session history. Use action_type to specify the operation.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action_type": {
                        "type": "string",
                        "enum": ["search", "neighbors", "path", "related", "decisions", "history", "stats", "index"],
                        "description": "The type of graph query to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query string (required for 'search' action)"
                    },
                    "category": {
                        "type": "string",
                        "description": "Filter by concept category (optional for 'search')",
                        "enum": ["architecture", "convention", "decision", "dependency", "domain_term", "error_class", "feature", "library", "pattern", "person", "platform", "process", "technology", "tool"]
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (optional)",
                        "minimum": 1
                    },
                    "node_id": {
                        "type": "string",
                        "description": "Node slug to explore (required for 'neighbors')"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Max traversal depth (optional for 'neighbors', default: 1)",
                        "minimum": 1
                    },
                    "edge_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by edge types (optional for 'neighbors')"
                    },
                    "from": {
                        "type": "string",
                        "description": "Source node slug (required for 'path')"
                    },
                    "to": {
                        "type": "string",
                        "description": "Target node slug (required for 'path')"
                    },
                    "max_hops": {
                        "type": "integer",
                        "description": "Maximum path length (optional for 'path', default: 5)",
                        "minimum": 1
                    },
                    "topic": {
                        "type": "string",
                        "description": "Topic to find related concepts for (required for 'related')"
                    },
                    "min_strength": {
                        "type": "number",
                        "description": "Minimum relationship strength (optional for 'related', 0.0-1.0)"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Filter decisions by domain (optional for 'decisions')"
                    },
                    "status": {
                        "type": "string",
                        "description": "Filter decisions by status (optional for 'decisions')",
                        "enum": ["active", "proposed", "reversed", "superseded"]
                    },
                    "since": {
                        "type": "string",
                        "description": "ISO timestamp — only return decisions after this date (optional for 'decisions')"
                    },
                    "concept": {
                        "type": "string",
                        "description": "Concept slug to get history for (required for 'history')"
                    },
                    "scope": {
                        "type": "string",
                        "description": "Indexing scope — 'current' for current session, 'all' for all unindexed (optional for 'index')"
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
            &serde_json::json!({"action_type": format!("{:?}", args.action)}),
        ) {
            return Err(ToolError::Blocked {
                tool: "graph_search",
                message: reason,
            });
        }

        let result = execute_graph_search(self.session_id, args.action);
        Ok(result)
    }
}
