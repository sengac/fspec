//! AST Call Chain — Path Tracing Between Two Functions
//!
//! Finds the call chains between two functions over the Calls adjacency of
//! the graph's query index (KGRAPH-077): one BFS for distances, then the
//! chains to each direct caller of the target in slug order (see
//! [`bfs::find_paths`]).
//!
//! Returns chains ordered by length, then by their slug sequence, at most
//! 20, with `truncated` when more exist. Each chain includes
//! function_chain (node metadata), call_details (one entry per hop) and
//! chain_length — mirroring CGC's find_function_call_chain() return
//! structure.

pub mod bfs;

use crate::database::GraphDatabase;
use crate::dispatch_helpers::{error_result, stamp};
use crate::query_index::{with_source, QueryIndex};
use serde_json::Value;
use std::collections::HashMap;

/// Maximum number of chains to return.
const MAX_CHAINS: usize = 20;

/// Default max BFS depth.
const DEFAULT_MAX_DEPTH: u32 = 5;

const ACTION: &str = "ast_call_chain";

/// Dispatch handler for the `ast_call_chain` action.
pub async fn dispatch_ast_call_chain(
    db: &GraphDatabase,
    from_slug: &str,
    to_slug: &str,
    max_depth: Option<u32>,
) -> String {
    match call_chain(db, from_slug, to_slug, max_depth).await {
        Ok(result) => stamp(db, result),
        Err(error) => error_result(db, ACTION, error),
    }
}

async fn call_chain(
    db: &GraphDatabase,
    from_slug: &str,
    to_slug: &str,
    max_depth: Option<u32>,
) -> Result<Value, String> {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH) as usize;
    let index = db.query_index().await?;
    for slug in [from_slug, to_slug] {
        if !index.functions.contains_key(slug) {
            return Err(format!("Function not found: {slug}"));
        }
    }

    let found = bfs::find_paths(
        &index.callees,
        &index.callers,
        from_slug,
        to_slug,
        depth_limit,
        MAX_CHAINS,
    );

    if found.chains.is_empty() {
        return Ok(serde_json::json!({
            "action": ACTION,
            "from": from_slug,
            "to": to_slug,
            "chains": [],
            "count": 0,
            "truncated": false,
            "message": format!("No call path found within depth {depth_limit}"),
        }));
    }

    let structured = build_structured_chains(db, &index, &found.chains).await?;
    Ok(serde_json::json!({
        "action": ACTION,
        "from": from_slug,
        "to": to_slug,
        "chains": structured,
        "count": structured.len(),
        "truncated": found.truncated,
        "summary": format!(
            "Found {} call chain(s) from '{}' to '{}' (max depth: {}){}",
            structured.len(), from_slug, to_slug, depth_limit,
            if found.truncated { "; more chains exist" } else { "" }
        ),
    }))
}

/// The metadata of a function a response returns: its index row, its file
/// path and its source text (fetched for this row only).
pub(crate) async fn function_row(
    db: &GraphDatabase,
    index: &QueryIndex,
    slug: &str,
) -> Result<Value, String> {
    let mut row = index
        .functions
        .get(slug)
        .cloned()
        .ok_or_else(|| format!("Function not found: {slug}"))?;
    if let (Value::Object(map), Some(path)) = (&mut row, index.function_path(slug)) {
        map.insert("path".to_string(), Value::String(path.to_string()));
    }
    with_source(db, "function_source", slug, row).await
}

/// Build structured chain objects mirroring CGC's return format.
///
/// Each chain has:
/// - `function_chain`: array of function metadata per node
/// - `call_details`: one entry per hop with its Calls edge's callCount and
///   isConditional (read from the edge rows, KGRAPH-078)
/// - `chain_length`: number of hops (edges)
async fn build_structured_chains(
    db: &GraphDatabase,
    index: &QueryIndex,
    chains: &[Vec<String>],
) -> Result<Vec<Value>, String> {
    let mut rows: HashMap<&str, Value> = HashMap::new();
    let mut structured = Vec::new();
    for chain in chains {
        let mut function_chain = Vec::new();
        for slug in chain {
            if !rows.contains_key(slug.as_str()) {
                rows.insert(slug, function_row(db, index, slug).await?);
            }
            function_chain.push(rows[slug.as_str()].clone());
        }
        let mut call_details = Vec::new();
        for pair in chain.windows(2) {
            let edge = index
                .call_edges
                .get(&(pair[0].clone(), pair[1].clone()))
                .ok_or_else(|| format!("No Calls edge row from {} to {}", pair[0], pair[1]))?;
            call_details.push(serde_json::json!({
                "from": pair[0],
                "to": pair[1],
                "callCount": edge.get("callCount").cloned().unwrap_or(Value::Null),
                "isConditional": edge.get("isConditional").cloned().unwrap_or(Value::Null),
            }));
        }
        structured.push(serde_json::json!({
            "function_chain": function_chain,
            "call_details": call_details,
            "chain_length": chain.len() - 1,
        }));
    }
    Ok(structured)
}
