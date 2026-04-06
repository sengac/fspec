//! GraphSearch Tool — Definition & Entry Point
//!
//! Provides the `GraphSearchTool` struct that implements the Rig `Tool` trait.
//! Supports the dual-graph architecture (AST + Learnings).

mod handler;
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect)]
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

/// GraphSearch tool — queries the dual-graph knowledge system.
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
                        "enum": ["ast_search", "ast_neighbors", "ast_stats", "ast_index", "ast_dead_code", "ast_call_chain", "ast_callers", "ast_callees", "ast_hierarchy", "ast_complexity", "ast_export", "ast_import", "learnings_search", "learnings_decisions", "learnings_stats", "learnings_related"],
                        "description": "The type of graph query to perform"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query string (required for 'ast_search' and 'learnings_search')"
                    },
                    "entity_type": {
                        "type": "string",
                        "description": "Filter AST search by entity type (optional for 'ast_search')",
                        "enum": ["Function", "File", "Type", "Dependency", "Variable"]
                    },
                    "path": {
                        "type": "string",
                        "description": "Glob filter to scope AST results to matching file paths (optional for 'ast_search' and 'ast_dead_code'). For 'ast_index', specifies a directory to index with .gitignore disabled (useful for indexing external repos in gitignored directories). Examples: 'src/tui/**/*.tsx', 'codelet/napi/src/**/*.rs', 'tmp/my-repo'"
                    },
                    "reset": {
                        "type": "boolean",
                        "description": "When true, deletes the existing on-disk graph database and clears the in-memory cache before re-indexing. Use after schema changes that make the existing database incompatible. Only applies to 'ast_index' action."
                    },
                    "incremental": {
                        "type": "boolean",
                        "description": "When true, only re-extracts files whose modification time has changed since the last index. Unchanged file entities are reused from the existing graph. Falls back to full extraction when no prior index exists or when >50% of files have changed. Only applies to 'ast_index' action."
                    },
                    "category": {
                        "type": "string",
                        "description": "Filter by learning category (optional for 'learnings_search')",
                        "enum": ["convention", "pattern", "anti_pattern", "decision", "discovery", "constraint", "reformulation"]
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (optional)",
                        "minimum": 1
                    },
                    "node_id": {
                        "type": "string",
                        "description": "Node slug to explore (required for 'ast_neighbors', 'ast_callers', 'ast_callees', 'ast_hierarchy'; optional for 'ast_complexity' — omit for top-N mode)"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Max traversal depth (optional for 'ast_neighbors', default: 1)",
                        "minimum": 1
                    },
                    "edge_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by edge types (optional for 'ast_neighbors')"
                    },
                    "topic": {
                        "type": "string",
                        "description": "Topic to find related learnings for (required for 'learnings_related')"
                    },
                    "min_strength": {
                        "type": "number",
                        "description": "Minimum relationship strength (optional for 'learnings_related', 0.0-1.0)"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Filter decisions by domain (optional for 'learnings_decisions')"
                    },
                    "status": {
                        "type": "string",
                        "description": "Filter decisions by status (optional for 'learnings_decisions')",
                        "enum": ["active", "proposed", "reversed", "superseded"]
                    },
                    "from": {
                        "type": "string",
                        "description": "Source function slug for call chain tracing (required for 'ast_call_chain')"
                    },
                    "to": {
                        "type": "string",
                        "description": "Target function slug for call chain tracing (required for 'ast_call_chain')"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum BFS traversal depth (optional for 'ast_call_chain', 'ast_callers', 'ast_callees', default: 5)",
                        "minimum": 1
                    },
                    "include_methods": {
                        "type": "boolean",
                        "description": "Whether to include methods in hierarchy results (optional for 'ast_hierarchy', default: true)"
                    },
                    "min_threshold": {
                        "type": "integer",
                        "description": "Minimum cyclomatic complexity threshold (optional for 'ast_complexity', only return functions >= this value)",
                        "minimum": 1
                    },
                    "output_path": {
                        "type": "string",
                        "description": "File path for the output .astbundle file (required for 'ast_export')"
                    },
                    "input_path": {
                        "type": "string",
                        "description": "File path to the .astbundle file to import (required for 'ast_import')"
                    },
                    "merge_mode": {
                        "type": "string",
                        "description": "Import mode: 'overwrite' (default — replaces all data) or 'merge' (upserts by slug key). Only for 'ast_import'.",
                        "enum": ["overwrite", "merge"]
                    },
                    "search_mode": {
                        "type": "string",
                        "description": "Search mode for 'ast_search': 'name' (default — searches name/slug/path/qualifiedName), 'content' (searches source/docstring), 'all' (searches every field). Omit for name-only search.",
                        "enum": ["name", "content", "all"]
                    },
                    "decorator": {
                        "type": "string",
                        "description": "Filter by decorator/annotation name (optional for 'ast_search'). Case-insensitive, strips leading @/#[ for cross-language matching. Example: 'Test' matches @Test, @test, #[test]"
                    },
                    "parameter": {
                        "type": "string",
                        "description": "Filter by parameter name (optional for 'ast_search'). Case-insensitive contains match on function parameter names. Example: 'request' matches functions with a 'request' parameter"
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
