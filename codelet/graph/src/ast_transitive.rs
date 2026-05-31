//! AST Transitive Callers / Callees — Multi-Hop BFS Traversal
//!
//! Finds all direct and transitive callers or callees of a function via
//! multi-hop CALLS edge traversal. Uses BFS with depth annotation.
//!
//! Reuses the BFS infrastructure from `ast_call_chain` (KGRAPH-060):
//! - `GraphSnapshot` for pre-fetched function data
//! - `build_adjacency_list` for forward call-graph edges
//! - `find_all_reachable` for depth-annotated BFS traversal
//! - `reverse_adjacency` to invert edges for caller direction
//!
//! CGC equivalent: `find_all_callers()` / `find_all_callees()`.

use crate::ast_call_chain::bfs::{find_all_reachable, reverse_adjacency};
use crate::ast_call_chain::{build_adjacency_list, snapshot::GraphSnapshot};
use crate::database::GraphDatabase;
use serde_json::Value;

/// Default max BFS depth for transitive traversal.
const DEFAULT_MAX_DEPTH: u32 = 5;

/// Default maximum number of results.
const DEFAULT_LIMIT: usize = 50;

/// Dispatch handler for the `ast_callers` action.
///
/// Finds all direct and transitive callers of a function using BFS over
/// reversed Calls edges. Each result is annotated with its hop distance.
///
/// Mirrors CGC's `find_all_callers()` return structure:
/// - `results`: flat array of function metadata + depth
/// - `summary`: human-readable count string
pub async fn dispatch_ast_callers(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> String {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH) as usize;
    let result_limit = limit.unwrap_or(DEFAULT_LIMIT);

    let snapshot = GraphSnapshot::load(db).await;

    if !snapshot.function_exists(node_id) {
        return serde_json::json!({
            "action": "ast_callers",
            "error": format!("Function not found: {node_id}"),
        })
        .to_string();
    }

    let forward_adj = build_adjacency_list(db, &snapshot).await;
    let reverse_adj = reverse_adjacency(&forward_adj);

    let reachable = find_all_reachable(&reverse_adj, node_id, depth_limit, result_limit);

    let results = enrich_results(&snapshot, &reachable);

    serde_json::json!({
        "action": "ast_callers",
        "node_id": node_id,
        "results": results,
        "count": results.len(),
        "summary": format!(
            "Found {} direct and indirect caller(s) of '{}'",
            results.len(), node_id
        ),
    })
    .to_string()
}

/// Dispatch handler for the `ast_callees` action.
///
/// Finds all direct and transitive callees of a function using BFS over
/// forward Calls edges. Each result is annotated with its hop distance.
///
/// Mirrors CGC's `find_all_callees()` return structure:
/// - `results`: flat array of function metadata + depth
/// - `summary`: human-readable count string
pub async fn dispatch_ast_callees(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> String {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH) as usize;
    let result_limit = limit.unwrap_or(DEFAULT_LIMIT);

    let snapshot = GraphSnapshot::load(db).await;

    if !snapshot.function_exists(node_id) {
        return serde_json::json!({
            "action": "ast_callees",
            "error": format!("Function not found: {node_id}"),
        })
        .to_string();
    }

    let forward_adj = build_adjacency_list(db, &snapshot).await;

    let reachable = find_all_reachable(&forward_adj, node_id, depth_limit, result_limit);

    let results = enrich_results(&snapshot, &reachable);

    serde_json::json!({
        "action": "ast_callees",
        "node_id": node_id,
        "results": results,
        "count": results.len(),
        "summary": format!(
            "Found {} direct and indirect callee(s) of '{}'",
            results.len(), node_id
        ),
    })
    .to_string()
}

/// Enrich reachable nodes with full function metadata from the snapshot.
///
/// Merges the BFS depth annotation and resolved file path into the
/// function metadata object so each result has slug, name, path,
/// lineStart, lineEnd, depth, etc. — matching CGC's output format.
fn enrich_results(
    snapshot: &GraphSnapshot,
    reachable: &[crate::ast_call_chain::bfs::ReachableNode],
) -> Vec<Value> {
    reachable
        .iter()
        .map(|node| {
            let mut meta = snapshot.get_metadata(&node.slug);
            if let Value::Object(ref mut map) = meta {
                map.insert("depth".to_string(), Value::Number(node.depth.into()));
                if let Some(path) = snapshot.get_file_path(&node.slug) {
                    map.insert("path".to_string(), Value::String(path.to_string()));
                }
            }
            meta
        })
        .collect()
}
