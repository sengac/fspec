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

use crate::ast_dispatch::AST_QUERIES;
use crate::database::GraphDatabase;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};

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
/// and methods (functions in the same file as the type).
///
/// Mirrors CGC's `find_class_hierarchy()` return structure.
pub async fn dispatch_ast_hierarchy(
    db: &GraphDatabase,
    node_id: &str,
    max_depth: Option<u32>,
    _include_methods: Option<bool>,
) -> String {
    let depth_limit = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);

    // Verify type exists by querying all_types and matching slug
    let type_info = match find_type(db, node_id).await {
        Some(info) => info,
        None => {
            return serde_json::json!({
                "action": "ast_hierarchy",
                "error": format!("Type not found: {node_id}"),
            })
            .to_string();
        }
    };

    // BFS upward: find parents via Extends edges
    let parents = bfs_traverse(db, node_id, "type_extends", depth_limit).await;

    // BFS downward: find children via reverse Extends edges
    let children = bfs_traverse(db, node_id, "type_extended_by", depth_limit).await;

    // Direct interfaces via Implements edges (single-hop only)
    let interfaces = query_single_hop(db, node_id, "type_implements").await;

    // Find methods: functions in the same file as this type
    let methods = find_methods_in_file(db, node_id).await;

    serde_json::json!({
        "action": "ast_hierarchy",
        "type": type_info,
        "parents": parents.iter().map(|n| serde_json::json!({
            "slug": n.slug,
            "name": n.name,
            "depth": n.depth,
            "via": "Extends",
        })).collect::<Vec<_>>(),
        "children": children.iter().map(|n| serde_json::json!({
            "slug": n.slug,
            "name": n.name,
            "depth": n.depth,
            "via": "Extends",
        })).collect::<Vec<_>>(),
        "interfaces": interfaces,
        "methods": methods,
        "summary": format!(
            "Type '{}' has {} parent(s), {} child(ren), {} interface(s), {} method(s)",
            type_info["name"].as_str().unwrap_or(node_id),
            parents.len(),
            children.len(),
            interfaces.len(),
            methods.len(),
        ),
    })
    .to_string()
}

/// Find a type by slug, returning its metadata.
async fn find_type(db: &GraphDatabase, slug: &str) -> Option<Value> {
    let all_types = db
        .query_with_source(AST_QUERIES, "all_types", None)
        .await
        .ok()?;

    if let Value::Array(types) = all_types {
        for t in types {
            if t.get("slug").and_then(|v| v.as_str()) == Some(slug) {
                return Some(t);
            }
        }
    }
    None
}

/// BFS traverse Extends (or reverse Extends) edges iteratively.
///
/// `query_name` should be one of:
/// - `"type_extends"` — find parents (outgoing Extends)
/// - `"type_extended_by"` — find children (incoming Extends)
async fn bfs_traverse(
    db: &GraphDatabase,
    start_slug: &str,
    query_name: &str,
    max_depth: u32,
) -> Vec<HierarchyNode> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut results: Vec<HierarchyNode> = Vec::new();

    visited.insert(start_slug.to_string());
    queue.push_back((start_slug.to_string(), 0));

    while let Some((current_slug, current_depth)) = queue.pop_front() {
        if current_depth >= max_depth {
            continue;
        }

        let params = serde_json::json!({ "slug": current_slug });
        let neighbors = match db
            .query_with_source(AST_QUERIES, query_name, Some(&params))
            .await
        {
            Ok(Value::Array(items)) => items,
            _ => continue,
        };

        for item in neighbors {
            let slug = match item.get("slug").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            if !visited.insert(slug.clone()) {
                continue;
            }

            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let depth = current_depth + 1;
            results.push(HierarchyNode {
                slug: slug.clone(),
                name,
                depth,
            });

            queue.push_back((slug, depth));
        }
    }

    results
}

/// Query a single-hop edge relationship (e.g., Implements).
async fn query_single_hop(db: &GraphDatabase, slug: &str, query_name: &str) -> Vec<Value> {
    let params = serde_json::json!({ "slug": slug });
    match db
        .query_with_source(AST_QUERIES, query_name, Some(&params))
        .await
    {
        Ok(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

/// Find methods (functions) in the same file as the given type.
///
/// Since our schema doesn't have Type→Function containment edges,
/// we find the file containing the type, then return all functions
/// in that file as an approximation of the type's methods.
async fn find_methods_in_file(db: &GraphDatabase, type_slug: &str) -> Vec<Value> {
    // Step 1: Find the file containing this type via type_container query
    let params = serde_json::json!({ "slug": type_slug });
    let file_slug = match db
        .query_with_source(AST_QUERIES, "type_container", Some(&params))
        .await
    {
        Ok(Value::Array(items)) => items
            .first()
            .and_then(|f| f.get("slug").and_then(|v| v.as_str()))
            .map(|s| s.to_string()),
        _ => None,
    };

    let file_slug = match file_slug {
        Some(s) => s,
        None => return Vec::new(),
    };

    // Step 2: Find all functions in that file via file_functions query
    let file_params = serde_json::json!({ "slug": file_slug });
    match db
        .query_with_source(AST_QUERIES, "file_functions", Some(&file_params))
        .await
    {
        Ok(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}
