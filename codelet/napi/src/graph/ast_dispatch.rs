//! AST Graph Dispatch Functions
//!
//! Implements query dispatch logic for AST-specific GraphSearch actions.
//! Routes queries to the AST graph database (separate from agent-memory graph).
//!
//! Each function takes a `GraphDatabase` reference and returns a JSON string.

use crate::graph::database::GraphDatabase;
use crate::graph::dispatch_helpers::{format_graph_stats, matches_fields, AST_SEARCHABLE_FIELDS};
use serde_json::Value;
use tracing::warn;

/// Bundled AST query source for named queries.
pub const AST_QUERIES: &str = include_str!("../../schemas/ast-queries.gq");

/// A named query with metadata about the edge type it represents.
struct NeighborQuery {
    query_name: &'static str,
    edge_type: &'static str,
}

/// All neighbor queries with their edge type metadata.
///
/// Since nanograph return clauses cannot include literal strings,
/// we inject the edge type metadata on the Rust side.
const NEIGHBOR_QUERIES: &[NeighborQuery] = &[
    // File neighbors
    NeighborQuery { query_name: "file_functions", edge_type: "Contains" },
    NeighborQuery { query_name: "file_types", edge_type: "ContainsType" },
    NeighborQuery { query_name: "file_imports", edge_type: "Imports" },
    NeighborQuery { query_name: "file_dependencies", edge_type: "DependsOn" },
    // Function neighbors
    NeighborQuery { query_name: "function_calls", edge_type: "Calls" },
    NeighborQuery { query_name: "function_callers", edge_type: "CalledBy" },
    NeighborQuery { query_name: "function_type_refs", edge_type: "TypeRef" },
    NeighborQuery { query_name: "function_container", edge_type: "ContainedBy" },
    // Type neighbors
    NeighborQuery { query_name: "type_container", edge_type: "ContainedBy" },
    NeighborQuery { query_name: "type_implements", edge_type: "Implements" },
    NeighborQuery { query_name: "type_extends", edge_type: "Extends" },
    NeighborQuery { query_name: "type_referencing_functions", edge_type: "ReferencedBy" },
];

/// Search AST code entities by name/pattern.
///
/// Supports filtering by entity_type ("Function", "File", "Type", "Dependency").
/// If no entity_type is specified, searches across all entity types.
/// Uses client-side filtering on the result sets.
pub async fn dispatch_ast_search(
    db: &GraphDatabase,
    query: &str,
    entity_type: Option<&str>,
    limit: Option<usize>,
) -> String {
    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(20);
    let mut results = Vec::new();

    let types_to_search: Vec<&str> = match entity_type {
        Some(t) => vec![t],
        None => vec!["Function", "File", "Type", "Dependency"],
    };

    for search_type in types_to_search {
        let query_name = match search_type {
            "Function" => "all_functions",
            "File" => "all_files",
            "Type" => "all_types",
            "Dependency" => "all_dependencies",
            _ => continue,
        };

        match db.query_with_source(AST_QUERIES, query_name, None).await {
            Ok(Value::Array(items)) => {
                for item in items {
                    if results.len() >= max_results {
                        break;
                    }
                    if matches_fields(&item, &query_lower, AST_SEARCHABLE_FIELDS) {
                        results.push(item);
                    }
                }
            }
            Ok(_) => { /* query returned non-array result; skip */ }
            Err(e) => {
                warn!(query_name, error = %e, "AST search query failed");
            }
        }
    }

    let count = results.len();
    serde_json::json!({
        "action": "ast_search",
        "query": query,
        "entity_type": entity_type,
        "results": results,
        "count": count,
    })
    .to_string()
}

/// Get neighbors of an AST node (callers, callees, containers, etc.).
///
/// Since nanograph requires typed variables and explicit edge names,
/// we compose neighbors from multiple per-edge-type queries. Each query
/// targets a specific edge type, and we inject the edge type metadata
/// into the results from the Rust side.
///
/// `_depth` and `_edge_types` are reserved for future traversal filtering
/// but not yet implemented in the nanograph query layer.
pub async fn dispatch_ast_neighbors(
    db: &GraphDatabase,
    node_id: &str,
    _depth: Option<usize>,
    _edge_types: Option<&[String]>,
) -> String {
    let params = serde_json::json!({ "slug": node_id });
    let mut neighbors = Vec::new();

    for nq in NEIGHBOR_QUERIES {
        match db
            .query_with_source(AST_QUERIES, nq.query_name, Some(&params))
            .await
        {
            Ok(Value::Array(items)) => {
                for mut item in items {
                    // Inject the edge type metadata since nanograph can't return literals
                    if let Value::Object(ref mut map) = item {
                        map.insert(
                            "edgeType".to_string(),
                            Value::String(nq.edge_type.to_string()),
                        );
                    }
                    neighbors.push(item);
                }
            }
            Ok(_) => { /* query returned non-array result; skip */ }
            Err(e) => {
                warn!(query_name = nq.query_name, error = %e, "AST neighbor query failed");
            }
        }
    }

    serde_json::json!({
        "action": "ast_neighbors",
        "node_id": node_id,
        "neighbors": neighbors,
        "count": neighbors.len(),
    })
    .to_string()
}

/// Get AST codebase statistics.
///
/// Uses the GraphDatabase's built-in `stats()` method for accurate counts
/// directly from storage segments.
pub async fn dispatch_ast_stats(db: &GraphDatabase) -> String {
    format_graph_stats(db, "ast_stats")
}
