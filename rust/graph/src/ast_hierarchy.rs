//! AST Class Hierarchy — Inheritance Traversal
//!
//! Finds parent classes, child classes, implemented interfaces,
//! and methods for a given type via iterative BFS over Extends
//! and Implements edges.
//!
//! Reuses the nanograph query infrastructure from `ast_dispatch`
//! (`type_extends`, `type_implements`, `type_container`,
//! `file_functions`) plus two new reverse queries
//! (`type_extended_by`, `type_implemented_by`).
//!
//! CGC equivalent: `find_class_hierarchy()` + `find_function_overrides()`.

use crate::database::GraphDatabase;
use crate::dispatch_helpers::{error_result, stamp};
use crate::query_index::{rows, with_source};
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

/// Default max BFS depth for hierarchy traversal.
const DEFAULT_MAX_DEPTH: u32 = 3;

/// A type node discovered during BFS, annotated with traversal depth.
struct HierarchyNode {
    slug: String,
    name: String,
    depth: u32,
}

/// Dispatch handler for the `ast_hierarchy` action.
///
/// Returns the full inheritance tree for a type: parents (via Extends),
/// children (via reverse Extends), implemented interfaces (via Implements),
/// and methods (via HasMethod). Each level and list is in slug order; a
/// failed query is an error, never an empty list (KGRAPH-077).
///
/// Mirrors CGC's `find_class_hierarchy()` return structure.
pub async fn dispatch_ast_hierarchy(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    _include_methods: Option<bool>,
) -> String {
    match hierarchy(db, node_id, max_depth.unwrap_or(DEFAULT_MAX_DEPTH)).await {
        Ok(result) => stamp(db, result),
        Err(error) => error_result(db, "ast_hierarchy", error),
    }
}

async fn hierarchy(db: &GraphDatabase, node_id: &str, depth_limit: u32) -> Result<Value, String> {
    let index = db.query_index().await?;
    let mut type_info = index
        .types
        .get(node_id)
        .cloned()
        .ok_or_else(|| format!("Type not found: {node_id}"))?;
    if let (Value::Object(map), Some(path)) = (&mut type_info, index.type_path(node_id)) {
        map.insert("path".to_string(), Value::String(path.to_string()));
    }
    let type_info = with_source(db, "type_source", node_id, type_info).await?;

    // BFS upward: find parents via Extends edges
    let parents = bfs_traverse(db, node_id, "type_extends", depth_limit).await?;

    // BFS downward: find children via reverse Extends edges
    let children = bfs_traverse(db, node_id, "type_extended_by", depth_limit).await?;

    // Direct interfaces via Implements edges (single-hop only)
    let interfaces = sorted_rows(db, node_id, "type_implements").await?;

    // The type's methods: the functions its HasMethod edges lead to (the
    // functions defined in its body or in its impl blocks, KGRAPH-075).
    let methods = sorted_rows(db, node_id, "type_methods").await?;

    let as_json = |nodes: &[HierarchyNode]| -> Vec<Value> {
        nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "slug": n.slug,
                    "name": n.name,
                    "depth": n.depth,
                    "via": "Extends",
                })
            })
            .collect()
    };
    Ok(serde_json::json!({
        "action": "ast_hierarchy",
        "summary": format!(
            "Type '{}' has {} parent(s), {} child(ren), {} interface(s), {} method(s)",
            type_info["name"].as_str().unwrap_or(node_id),
            parents.len(),
            children.len(),
            interfaces.len(),
            methods.len(),
        ),
        "type": type_info,
        "parents": as_json(&parents),
        "children": as_json(&children),
        "interfaces": interfaces,
        "methods": methods,
    }))
}

/// BFS over Extends (or reverse Extends) edges, one query per type reached,
/// each level in slug order.
///
/// `query_name` should be one of:
/// - `"type_extends"` — find parents (outgoing Extends)
/// - `"type_extended_by"` — find children (incoming Extends)
async fn bfs_traverse(
    db: &GraphDatabase,
    start_slug: &str,
    query_name: &str,
    max_depth: u32,
) -> Result<Vec<HierarchyNode>, String> {
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(start_slug.to_string());
    let mut frontier = vec![start_slug.to_string()];
    let mut results: Vec<HierarchyNode> = Vec::new();
    for depth in 1..=max_depth {
        let mut next: BTreeSet<(String, String)> = BTreeSet::new();
        for slug in &frontier {
            for item in sorted_rows(db, slug, query_name).await? {
                let neighbour = item.get("slug").and_then(Value::as_str).ok_or_else(|| {
                    format!("Graph query '{query_name}' returned a row without slug: {item}")
                })?;
                if !visited.contains(neighbour) {
                    let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
                    next.insert((neighbour.to_string(), name.to_string()));
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = Vec::new();
        for (slug, name) in next {
            if visited.insert(slug.clone()) {
                frontier.push(slug.clone());
                results.push(HierarchyNode { slug, name, depth });
            }
        }
    }
    Ok(results)
}

/// The rows of a single-hop query from `slug`, in slug order.
async fn sorted_rows(
    db: &GraphDatabase,
    slug: &str,
    query_name: &str,
) -> Result<Vec<Value>, String> {
    let params = serde_json::json!({ "slug": slug });
    let mut found = rows(db, query_name, Some(&params)).await?;
    found.sort_by(|a, b| {
        let key = |row: &Value| {
            row.get("slug")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        key(a).cmp(&key(b))
    });
    Ok(found)
}
