//! AST Call Chain — BFS Path Tracing Between Two Functions
//!
//! Finds the shortest call path(s) between two functions via multi-hop
//! CALLS edge traversal. Uses iterative BFS over single-hop nanograph
//! queries since nanograph doesn't support variable-length paths.
//!
//! Returns chains ordered by length (shortest first), limited to 20.
//! Each chain includes function_chain (node metadata), call_details
//! (edge metadata per hop), and chain_length — mirroring CGC's
//! find_function_call_chain() return structure.

pub mod bfs;
pub mod snapshot;

use crate::graph::ast_dispatch::AST_QUERIES;
use crate::graph::database::GraphDatabase;
use bfs::{AdjEntry, CallEdgeInfo};
use serde_json::Value;
use std::collections::HashMap;

/// Maximum number of chains to return.
const MAX_CHAINS: usize = 20;

/// Default max BFS depth.
const DEFAULT_MAX_DEPTH: u32 = 5;

/// Dispatch handler for the `ast_call_chain` action.
///
/// Finds all call chains from `from_slug` to `to_slug` using BFS,
/// returning the shortest chains first. Each chain includes
/// `function_chain`, `call_details`, and `chain_length` — mirroring
/// CGC's `find_function_call_chain()` return structure.
pub async fn dispatch_ast_call_chain(
    db: &GraphDatabase,
    from_slug: &str,
    to_slug: &str,
    max_depth: Option<u32>,
) -> String {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH) as usize;

    // Load all function data once upfront to avoid redundant queries.
    let graph_snapshot = snapshot::GraphSnapshot::load(db).await;

    // Validate source function exists
    if !graph_snapshot.function_exists(from_slug) {
        return serde_json::json!({
            "action": "ast_call_chain",
            "error": format!("Function not found: {from_slug}"),
        })
        .to_string();
    }

    // Validate target function exists
    if !graph_snapshot.function_exists(to_slug) {
        return serde_json::json!({
            "action": "ast_call_chain",
            "error": format!("Function not found: {to_slug}"),
        })
        .to_string();
    }

    // Build adjacency list with edge metadata from per-function call queries.
    let adj = build_adjacency_list(db, &graph_snapshot).await;

    // Run BFS to find all paths from source to target
    let chains = bfs::find_paths(&adj, from_slug, to_slug, depth_limit, MAX_CHAINS);

    if chains.is_empty() {
        return serde_json::json!({
            "action": "ast_call_chain",
            "from": from_slug,
            "to": to_slug,
            "chains": [],
            "count": 0,
            "message": format!("No call path found within depth {depth_limit}"),
        })
        .to_string();
    }

    // Build structured chain objects with function_chain + call_details
    let structured = build_structured_chains(&graph_snapshot, &adj, &chains);

    serde_json::json!({
        "action": "ast_call_chain",
        "from": from_slug,
        "to": to_slug,
        "chains": structured,
        "count": structured.len(),
        "summary": format!(
            "Found {} call chain(s) from '{}' to '{}' (max depth: {})",
            structured.len(), from_slug, to_slug, depth_limit
        ),
    })
    .to_string()
}

/// Build an adjacency list from the function_calls query for all functions.
///
/// Public so it can be reused by `ast_transitive` (KGRAPH-061).
pub async fn build_adjacency_list(
    db: &GraphDatabase,
    graph_snapshot: &snapshot::GraphSnapshot,
) -> HashMap<String, Vec<AdjEntry>> {
    let mut adj: HashMap<String, Vec<AdjEntry>> = HashMap::new();

    for slug in graph_snapshot.known_slugs() {
        let params = serde_json::json!({ "slug": slug });
        if let Ok(Value::Array(callees)) = db
            .query_with_source(AST_QUERIES, "function_calls", Some(&params))
            .await
        {
            let entries: Vec<AdjEntry> = callees
                .iter()
                .filter_map(|c| {
                    let callee_slug = c.get("slug")
                        .and_then(|v| v.as_str())
                        .map(String::from)?;
                    let edge_info = CallEdgeInfo {
                        call_count: c.get("callCount").and_then(|v| v.as_i64()),
                        is_conditional: c.get("isConditional").and_then(|v| v.as_bool()),
                    };
                    Some(AdjEntry { callee_slug, edge_info })
                })
                .collect();
            if !entries.is_empty() {
                adj.insert(slug.clone(), entries);
            }
        }
    }

    adj
}

/// Build structured chain objects mirroring CGC's return format.
///
/// Each chain has:
/// - `function_chain`: array of function metadata per node
/// - `call_details`: array of edge metadata per hop
/// - `chain_length`: number of hops (edges)
fn build_structured_chains(
    graph_snapshot: &snapshot::GraphSnapshot,
    adj: &HashMap<String, Vec<AdjEntry>>,
    chains: &[Vec<String>],
) -> Vec<Value> {
    chains
        .iter()
        .map(|chain| {
            let function_chain: Vec<Value> = chain
                .iter()
                .map(|slug| {
                    let mut meta = graph_snapshot.get_metadata(slug);
                    if let Value::Object(ref mut map) = meta {
                        if let Some(path) = graph_snapshot.get_file_path(slug) {
                            map.insert("path".to_string(), Value::String(path.to_string()));
                        }
                    }
                    meta
                })
                .collect();

            let call_details: Vec<Value> = chain
                .windows(2)
                .map(|pair| {
                    let from = &pair[0];
                    let to = &pair[1];
                    let edge = adj
                        .get(from)
                        .and_then(|entries| {
                            entries.iter().find(|e| e.callee_slug == *to)
                        });
                    match edge {
                        Some(e) => serde_json::json!({
                            "from": from,
                            "to": to,
                            "callCount": e.edge_info.call_count,
                            "isConditional": e.edge_info.is_conditional,
                        }),
                        None => serde_json::json!({
                            "from": from,
                            "to": to,
                        }),
                    }
                })
                .collect();

            serde_json::json!({
                "function_chain": function_chain,
                "call_details": call_details,
                "chain_length": chain.len() - 1,
            })
        })
        .collect()
}
