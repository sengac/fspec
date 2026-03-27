//! AST Graph Dispatch Functions
//!
//! Routes queries to the AST code graph database (dual-graph architecture).
//! Each function takes a `GraphDatabase` reference and returns a JSON string.
//!
//! Note: `dispatch_ast_index` lives in `ast_index.rs` and
//! `dispatch_ast_dead_code` lives in `ast_dead_code.rs` (extracted for
//! file-size compliance).

use crate::graph::database::GraphDatabase;
use crate::graph::dispatch_helpers::{
    fields_for_search_mode, format_graph_stats, matches_decorator, matches_fields,
    matches_parameter,
};
use globset::{Glob, GlobMatcher};
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
    NeighborQuery { query_name: "file_variables", edge_type: "ContainsVariable" },
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
    // Variable neighbors
    NeighborQuery { query_name: "variable_container", edge_type: "ContainedBy" },
];

/// Build a compiled glob matcher from an optional path pattern.
/// Returns `None` if no pattern or if the pattern is invalid.
pub(crate) fn build_glob_matcher(path_pattern: Option<&str>) -> Option<GlobMatcher> {
    path_pattern.and_then(|pattern| {
        match Glob::new(pattern) {
            Ok(g) => Some(g.compile_matcher()),
            Err(e) => {
                warn!(pattern, error = %e, "Invalid glob pattern for path filter — ignoring");
                None
            }
        }
    })
}

/// Check if a JSON item matches a path glob filter.
///
/// For File entities: checks the "path" field directly.
/// For Function/Type entities: checks the "qualifiedName" or "slug" field
/// to extract the file slug prefix, then looks up the file path via the
/// `file_paths` map. If no file_paths map is provided, falls back to
/// matching the slug prefix against the pattern.
pub(crate) fn matches_path_glob(item: &Value, matcher: &GlobMatcher, path_field: &str) -> bool {
    item.get(path_field)
        .and_then(|v| v.as_str())
        .is_some_and(|path| matcher.is_match(path))
}

/// Look up all file paths from the graph, returning a slug → path map.
///
/// Used to resolve Function/Type slugs (which contain the file slug prefix)
/// back to the actual file path for glob matching.
pub(crate) async fn build_file_path_index(
    db: &GraphDatabase,
) -> std::collections::HashMap<String, String> {
    let mut index = std::collections::HashMap::new();
    if let Ok(Value::Array(files)) = db.query_with_source(AST_QUERIES, "all_files", None).await {
        for file in files {
            if let (Some(slug), Some(path)) = (
                file.get("slug").and_then(|v| v.as_str()),
                file.get("path").and_then(|v| v.as_str()),
            ) {
                index.insert(slug.to_string(), path.to_string());
            }
        }
    }
    index
}

/// Check if a Function or Type item matches a path glob by resolving its
/// file slug from the qualifiedName (format: "file-slug::entityName").
pub(crate) fn matches_entity_path_glob(
    item: &Value,
    matcher: &GlobMatcher,
    file_path_index: &std::collections::HashMap<String, String>,
) -> bool {
    // Try qualifiedName first (Functions), then slug (Types)
    let id = item
        .get("qualifiedName")
        .or_else(|| item.get("slug"))
        .and_then(|v| v.as_str());

    if let Some(id_str) = id {
        // Split on "::" to get the file slug prefix
        if let Some(file_slug) = id_str.split("::").next() {
            if let Some(file_path) = file_path_index.get(file_slug) {
                return matcher.is_match(file_path);
            }
        }
    }
    false
}

/// Search AST code entities by name/pattern.
///
/// Supports filtering by entity_type ("Function", "File", "Type", "Dependency").
/// If no entity_type is specified, searches across all entity types.
/// Uses client-side filtering on the result sets.
///
/// Optional `path_pattern` applies a glob filter to scope results to
/// entities from matching file paths.
///
/// Optional `search_mode` controls which fields are searched:
/// - `"name"` (default) — name, slug, path, qualifiedName
/// - `"content"` — source, docstring only
/// - `"all"` — all fields including source, docstring, parameters, decorators
///
/// Optional `decorator_filter` restricts results to entities whose decorators
/// match (case-insensitive, cross-language symbol stripping).
///
/// Optional `parameter_filter` restricts results to functions whose parameter
/// list contains the given name (case-insensitive).
pub async fn dispatch_ast_search(
    db: &GraphDatabase,
    query: &str,
    entity_type: Option<&str>,
    limit: Option<usize>,
    path_pattern: Option<&str>,
    search_mode: Option<&str>,
    decorator_filter: Option<&str>,
    parameter_filter: Option<&str>,
) -> String {
    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(20);
    let mut results = Vec::new();

    let glob_matcher = build_glob_matcher(path_pattern);
    let search_fields = fields_for_search_mode(search_mode);

    // Build file path index if we need to resolve Function/Type paths
    let file_path_index = if glob_matcher.is_some() {
        build_file_path_index(db).await
    } else {
        std::collections::HashMap::new()
    };

    let types_to_search: Vec<&str> = match entity_type {
        Some(t) => vec![t],
        None => vec!["Function", "File", "Type", "Dependency", "Variable"],
    };

    for search_type in types_to_search {
        let query_name = match search_type {
            "Function" => "all_functions",
            "File" => "all_files",
            "Type" => "all_types",
            "Dependency" => "all_dependencies",
            "Variable" => "all_variables",
            _ => continue,
        };

        match db.query_with_source(AST_QUERIES, query_name, None).await {
            Ok(Value::Array(items)) => {
                for item in items {
                    if results.len() >= max_results {
                        break;
                    }
                    // Text match: skip if query is non-empty and no field matches
                    if !query_lower.is_empty()
                        && !matches_fields(&item, &query_lower, search_fields)
                    {
                        continue;
                    }
                    // Decorator filter: AND constraint
                    if let Some(dec_filter) = decorator_filter {
                        if !matches_decorator(&item, dec_filter) {
                            continue;
                        }
                    }
                    // Parameter filter: AND constraint
                    if let Some(param_filter) = parameter_filter {
                        if !matches_parameter(&item, param_filter) {
                            continue;
                        }
                    }
                    // Apply glob filter if provided
                    if let Some(ref matcher) = glob_matcher {
                        let matches = match search_type {
                            "File" => matches_path_glob(&item, matcher, "path"),
                            "Variable" => matches_path_glob(&item, matcher, "path"),
                            "Function" | "Type" => {
                                matches_entity_path_glob(&item, matcher, &file_path_index)
                            }
                            _ => true, // Dependencies don't have file paths
                        };
                        if !matches {
                            continue;
                        }
                    }
                    results.push(item);
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
        "search_mode": search_mode.unwrap_or("name"),
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
