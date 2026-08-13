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
            let nodes = stats
                .get("nodes")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let edges_obj = stats
                .get("edges")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

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
        Err(e) => serde_json::json!({
            "action": action_name,
            "error": format!("Failed to get stats: {e}"),
        })
        .to_string(),
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
pub const AST_SEARCHABLE_FIELDS: &[&str] = &[
    "name",
    "slug",
    "path",
    "qualifiedName",
    "source",
    "docstring",
    "parameters",
    "decorators",
];

/// Searchable fields for AST name-only mode (default).
///
/// Only searches structural identifiers — excludes source code, docstrings,
/// parameters, and decorators to avoid noisy matches.
pub const AST_NAME_FIELDS: &[&str] = &["name", "slug", "path", "qualifiedName"];

/// Searchable fields for AST content-only mode.
///
/// Searches inside function bodies and documentation only — excludes names
/// so agents can find code by what it does, not what it's called.
pub const AST_CONTENT_FIELDS: &[&str] = &["source", "docstring"];

/// Select the appropriate field list for a given search mode.
///
/// Returns the field slice corresponding to the mode string:
/// - `"name"` (or `None`) → `AST_NAME_FIELDS`
/// - `"content"` → `AST_CONTENT_FIELDS`
/// - `"all"` → `AST_SEARCHABLE_FIELDS`
pub fn fields_for_search_mode(mode: Option<&str>) -> &'static [&'static str] {
    match mode {
        Some("content") => AST_CONTENT_FIELDS,
        Some("all") => AST_SEARCHABLE_FIELDS,
        _ => AST_NAME_FIELDS, // "name" or None → default
    }
}

/// Check if an entity's `decorators` field matches a filter string.
///
/// Performs case-insensitive matching, stripping leading `@`, `#[`, and `]`
/// from each decorator token for cross-language compatibility.
/// E.g., filter `"test"` matches `"@Test"`, `"#[test]"`, `"@test"`.
pub fn matches_decorator(item: &Value, decorator_filter: &str) -> bool {
    let filter_lower = decorator_filter.to_lowercase();
    let filter_stripped = filter_lower
        .trim_start_matches('@')
        .trim_start_matches("#[")
        .trim_end_matches(']');

    item.get("decorators")
        .and_then(|v| v.as_str())
        .is_some_and(|decorators_str| {
            decorators_str.split(',').any(|token| {
                let token_stripped = token.trim().to_lowercase();
                let token_clean = token_stripped
                    .trim_start_matches('@')
                    .trim_start_matches("#[")
                    .trim_end_matches(']')
                    .trim_end_matches(')');
                token_clean.contains(filter_stripped)
            })
        })
}

/// Check if an entity's `parameters` field contains a parameter name.
///
/// Performs case-insensitive matching on the comma-separated parameter list.
/// E.g., filter `"request"` matches `"self, request, response"`.
pub fn matches_parameter(item: &Value, parameter_filter: &str) -> bool {
    let filter_lower = parameter_filter.to_lowercase();
    item.get("parameters")
        .and_then(|v| v.as_str())
        .is_some_and(|params_str| {
            params_str
                .split(',')
                .any(|param| param.trim().to_lowercase().contains(&filter_lower))
        })
}

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
