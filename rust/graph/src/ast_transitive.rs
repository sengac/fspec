//! AST Transitive Callers / Callees — Multi-Hop BFS Traversal
//!
//! Finds all direct and transitive callers or callees of a function by BFS
//! over the Calls adjacency of the graph's query index (KGRAPH-077), with
//! each result annotated with its depth.
//!
//! nanograph's native `calls{1,N}` traversal answers reachability but
//! returns no hop count; the cached adjacency gives the depth of every row.
//! Results come in (depth, slug) order; a limit keeps the first results of
//! that order and `total` counts all of them.
//!
//! CGC equivalent: `find_all_callers()` / `find_all_callees()`.

use crate::ast_call_chain::bfs::{find_all_reachable, Adjacency};
use crate::ast_call_chain::function_row;
use crate::database::GraphDatabase;
use crate::dispatch_helpers::{error_result, stamp};
use serde_json::Value;

/// Default max BFS depth for transitive traversal.
const DEFAULT_MAX_DEPTH: u32 = 5;

/// Default maximum number of results.
const DEFAULT_LIMIT: usize = 50;

#[derive(Clone, Copy)]
enum Direction {
    Callers,
    Callees,
}

impl Direction {
    fn action(self) -> &'static str {
        match self {
            Direction::Callers => "ast_callers",
            Direction::Callees => "ast_callees",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Direction::Callers => "caller",
            Direction::Callees => "callee",
        }
    }
}

/// Dispatch handler for the `ast_callers` action: every direct and
/// transitive caller of a function, over reversed Calls edges.
pub async fn dispatch_ast_callers(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> String {
    dispatch(db, Direction::Callers, node_id, max_depth, limit).await
}

/// Dispatch handler for the `ast_callees` action: every direct and
/// transitive callee of a function, over forward Calls edges.
pub async fn dispatch_ast_callees(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> String {
    dispatch(db, Direction::Callees, node_id, max_depth, limit).await
}

async fn dispatch(
    db: &GraphDatabase,
    direction: Direction,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> String {
    match transitive(db, direction, node_id, max_depth, limit).await {
        Ok(result) => stamp(db, result),
        Err(error) => error_result(db, direction.action(), error),
    }
}

async fn transitive(
    db: &GraphDatabase,
    direction: Direction,
    node_id: &str,
    max_depth: Option<u32>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH) as usize;
    let result_limit = limit.unwrap_or(DEFAULT_LIMIT);
    let index = db.query_index().await?;
    if !index.functions.contains_key(node_id) {
        return Err(format!("Function not found: {node_id}"));
    }
    let adjacency: &Adjacency = match direction {
        Direction::Callers => &index.callers,
        Direction::Callees => &index.callees,
    };
    let reachable = find_all_reachable(adjacency, node_id, depth_limit);
    let total = reachable.len();
    let mut results = Vec::new();
    for node in reachable.iter().take(result_limit) {
        let mut row = function_row(db, &index, &node.slug).await?;
        if let Value::Object(map) = &mut row {
            map.insert("depth".to_string(), Value::Number(node.depth.into()));
        }
        results.push(row);
    }
    Ok(serde_json::json!({
        "action": direction.action(),
        "node_id": node_id,
        "results": results,
        "count": results.len(),
        "total": total,
        "summary": format!(
            "Found {total} direct and indirect {}(s) of '{node_id}'; returned {}",
            direction.noun(), results.len()
        ),
    }))
}
