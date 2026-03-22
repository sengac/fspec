//! Shared dispatch helper functions for graph query dispatchers.
//!
//! Provides common utilities used by `ast_dispatch`, `learnings_dispatch`,
//! and any future graph-specific dispatchers to avoid code duplication.

use serde_json::Value;

use super::database::GraphDatabase;

/// Get graph statistics from a `GraphDatabase` instance, formatted as a JSON string.
///
/// Produces a standardized response with node counts per type and edge counts
/// (including a calculated total). Used by both AST and Learnings dispatchers.
pub fn format_graph_stats(db: &GraphDatabase, action_name: &str) -> String {
    match db.stats() {
        Ok(stats) => {
            let nodes = stats.get("nodes").cloned().unwrap_or(serde_json::json!({}));
            let edges_obj = stats.get("edges").cloned().unwrap_or(serde_json::json!({}));

            // Calculate total edges
            let total_edges: u64 = edges_obj
                .as_object()
                .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum())
                .unwrap_or(0);

            let mut edges = edges_obj.as_object().cloned().unwrap_or_default();
            edges.insert("total".to_string(), Value::Number(total_edges.into()));

            serde_json::json!({
                "action": action_name,
                "nodes": nodes,
                "edges": edges,
            })
            .to_string()
        }
        Err(e) => {
            serde_json::json!({
                "action": action_name,
                "error": format!("Failed to get stats: {e}"),
            })
            .to_string()
        }
    }
}

/// Check if a JSON value's searchable fields match a lowercased query string.
///
/// Performs a case-insensitive substring search across the specified fields.
/// Returns `true` if any field value contains the query string.
pub fn matches_fields(item: &Value, query_lower: &str, fields: &[&str]) -> bool {
    fields.iter().any(|field| {
        item.get(*field)
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.to_lowercase().contains(query_lower))
    })
}

/// Searchable fields for AST code entities.
pub const AST_SEARCHABLE_FIELDS: &[&str] = &["name", "slug", "path", "qualifiedName"];

/// Searchable fields for Learnings entities.
///
/// Used by `learnings_dispatch`, `learnings_context`, and any future modules
/// that need to search across Learnings node properties.
pub const LEARNINGS_SEARCHABLE_FIELDS: &[&str] = &[
    "title",
    "slug",
    "content",
    "description",
    "name",
    "rationale",
    "strategy",
    "domain",
];
