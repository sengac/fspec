#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! AST Graph Dispatch Functions
//!
//! Routes queries to the AST code graph database (dual-graph architecture).
//! Each function takes a `GraphDatabase` reference and returns a JSON string.
//!
//! Note: `dispatch_ast_index` lives in `ast_index.rs` and
//! `dispatch_ast_dead_code` lives in `ast_dead_code.rs` (extracted for
//! file-size compliance).

use crate::database::GraphDatabase;
use crate::dispatch_helpers::{
    error_result, fields_for_search_mode, graph_stats, matches_decorator, matches_fields,
    matches_parameter, stamp,
};
use crate::query_index::{rows, with_source, QueryIndex};
use globset::{Glob, GlobMatcher};
use serde_json::Value;

/// Bundled AST query source for named queries.
pub const AST_QUERIES: &str = include_str!("../schemas/ast-queries.gq");

/// The node type a neighbour query starts from (KGRAPH-077: only the
/// queries of the node's own type run).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    File,
    Function,
    Type,
    Variable,
}

/// A named query with metadata about the edge type it represents.
struct NeighborQuery {
    kind: NodeKind,
    query_name: &'static str,
    edge_type: &'static str,
}

/// All neighbor queries with their edge type metadata.
///
/// Since nanograph return clauses cannot include literal strings,
/// we inject the edge type metadata on the Rust side.
const NEIGHBOR_QUERIES: &[NeighborQuery] = &[
    // File neighbors
    NeighborQuery {
        kind: NodeKind::File,
        query_name: "file_functions",
        edge_type: "Contains",
    },
    NeighborQuery {
        kind: NodeKind::File,
        query_name: "file_types",
        edge_type: "ContainsType",
    },
    NeighborQuery {
        kind: NodeKind::File,
        query_name: "file_imports",
        edge_type: "Imports",
    },
    NeighborQuery {
        kind: NodeKind::File,
        query_name: "file_dependencies",
        edge_type: "DependsOn",
    },
    NeighborQuery {
        kind: NodeKind::File,
        query_name: "file_variables",
        edge_type: "ContainsVariable",
    },
    // Function neighbors
    NeighborQuery {
        kind: NodeKind::Function,
        query_name: "function_calls",
        edge_type: "Calls",
    },
    NeighborQuery {
        kind: NodeKind::Function,
        query_name: "function_callers",
        edge_type: "CalledBy",
    },
    NeighborQuery {
        kind: NodeKind::Function,
        query_name: "function_type_refs",
        edge_type: "TypeRef",
    },
    NeighborQuery {
        kind: NodeKind::Function,
        query_name: "function_container",
        edge_type: "ContainedBy",
    },
    // Type neighbors
    NeighborQuery {
        kind: NodeKind::Type,
        query_name: "type_container",
        edge_type: "ContainedBy",
    },
    NeighborQuery {
        kind: NodeKind::Type,
        query_name: "type_implements",
        edge_type: "Implements",
    },
    NeighborQuery {
        kind: NodeKind::Type,
        query_name: "type_extends",
        edge_type: "Extends",
    },
    NeighborQuery {
        kind: NodeKind::Type,
        query_name: "type_referencing_functions",
        edge_type: "ReferencedBy",
    },
    // Variable neighbors
    NeighborQuery {
        kind: NodeKind::Variable,
        query_name: "variable_container",
        edge_type: "ContainedBy",
    },
];

/// Compile the glob of an optional path pattern; an invalid pattern is an
/// error (KGRAPH-077), never an unfiltered result.
pub(crate) fn build_glob_matcher(
    path_pattern: Option<&str>,
) -> Result<Option<GlobMatcher>, String> {
    path_pattern
        .map(|pattern| {
            Glob::new(pattern)
                .map(|glob| glob.compile_matcher())
                .map_err(|e| format!("Invalid path glob '{pattern}': {e}"))
        })
        .transpose()
}

/// Whether the path of an entity (from its index file or its own `path`)
/// matches the glob; an entity without a path never matches.
fn path_matches(path: Option<&str>, matcher: &GlobMatcher) -> bool {
    path.is_some_and(|path| matcher.is_match(path))
}

/// The search filters of one ast_search request.
struct SearchFilters<'a> {
    query_lower: String,
    fields: &'static [&'static str],
    decorator: Option<&'a str>,
    parameter: Option<&'a str>,
    glob: Option<GlobMatcher>,
}

impl SearchFilters<'_> {
    fn matches(&self, item: &Value, path: Option<&str>, has_path: bool) -> bool {
        if !self.query_lower.is_empty() && !matches_fields(item, &self.query_lower, self.fields) {
            return false;
        }
        if self.decorator.is_some_and(|d| !matches_decorator(item, d)) {
            return false;
        }
        if self.parameter.is_some_and(|p| !matches_parameter(item, p)) {
            return false;
        }
        match &self.glob {
            Some(matcher) if has_path => path_matches(path, matcher),
            _ => true,
        }
    }
}

fn slug_of(row: &Value) -> &str {
    row.get("slug").and_then(Value::as_str).unwrap_or_default()
}

fn sorted_by_slug(mut rows: Vec<Value>) -> Vec<Value> {
    rows.sort_by(|a, b| slug_of(a).cmp(slug_of(b)));
    rows
}

/// Search AST code entities by name/pattern.
///
/// Supports filtering by entity_type ("Function", "File", "Type",
/// "Dependency", "Variable"); without one, every type is searched in that
/// order. Each type's matches come in slug order (KGRAPH-077); the result
/// holds the first `limit` matches (`count`) and the number of all matches
/// (`total`).
///
/// `search_mode` controls which fields are searched:
/// - `"name"` (default) — name, slug, path, qualifiedName. Functions and
///   types are matched in the query index, without source text; only the
///   rows returned get their source.
/// - `"content"` — source, docstring only
/// - `"all"` — all fields including source, docstring, parameters, decorators
///
/// `path_pattern` is a glob over the entity's file path (an invalid glob is
/// an error); `decorator_filter` and `parameter_filter` narrow further.
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
    let max_results = limit.unwrap_or(20);
    let result = async {
        let filters = SearchFilters {
            query_lower: query.to_lowercase(),
            fields: fields_for_search_mode(search_mode),
            decorator: decorator_filter,
            parameter: parameter_filter,
            glob: build_glob_matcher(path_pattern)?,
        };
        let reads_source = !matches!(search_mode, None | Some("name"));
        let index = db.query_index().await?;
        let types_to_search: Vec<&str> = match entity_type {
            Some(t) => vec![t],
            None => vec!["Function", "File", "Type", "Dependency", "Variable"],
        };
        let mut results = Vec::new();
        let mut total = 0usize;
        for search_type in types_to_search {
            let matched = search_type_rows(db, &index, search_type, reads_source, &filters).await?;
            total += matched.len();
            for (source_query, row) in matched {
                if results.len() >= max_results {
                    break;
                }
                results.push(match source_query {
                    Some(source_query) => {
                        let slug = slug_of(&row).to_string();
                        with_source(db, source_query, &slug, row).await?
                    }
                    None => row,
                });
            }
        }
        Ok::<_, String>((results, total))
    }
    .await;
    match result {
        Ok((results, total)) => stamp(
            db,
            serde_json::json!({
                "action": "ast_search",
                "query": query,
                "entity_type": entity_type,
                "search_mode": search_mode.unwrap_or("name"),
                "count": results.len(),
                "total": total,
                "results": results,
            }),
        ),
        Err(error) => error_result(db, "ast_search", error),
    }
}

/// The matching rows of one entity type in slug order, each with the query
/// that fetches its source when the row has none yet.
async fn search_type_rows(
    db: &GraphDatabase,
    index: &QueryIndex,
    search_type: &str,
    reads_source: bool,
    filters: &SearchFilters<'_>,
) -> Result<Vec<(Option<&'static str>, Value)>, String> {
    let tagged = |rows: Vec<Value>, source: Option<&'static str>| {
        rows.into_iter().map(move |row| (source, row))
    };
    let matched: Vec<(Option<&'static str>, Value)> = match (search_type, reads_source) {
        ("Function", false) => tagged(
            index
                .functions
                .iter()
                .filter(|(slug, row)| filters.matches(row, index.function_path(slug), true))
                .map(|(_, row)| row.clone())
                .collect(),
            Some("function_source"),
        )
        .collect(),
        ("Type", false) => tagged(
            index
                .types
                .iter()
                .filter(|(slug, row)| filters.matches(row, index.type_path(slug), true))
                .map(|(_, row)| row.clone())
                .collect(),
            Some("type_source"),
        )
        .collect(),
        ("Function", true) => tagged(
            sorted_by_slug(rows(db, "all_functions", None).await?)
                .into_iter()
                .filter(|row| filters.matches(row, index.function_path(slug_of(row)), true))
                .collect(),
            None,
        )
        .collect(),
        ("Type", true) => tagged(
            sorted_by_slug(rows(db, "all_types", None).await?)
                .into_iter()
                .filter(|row| filters.matches(row, index.type_path(slug_of(row)), true))
                .collect(),
            None,
        )
        .collect(),
        ("File", _) => tagged(
            index
                .files
                .values()
                .filter(|row| filters.matches(row, row.get("path").and_then(Value::as_str), true))
                .cloned()
                .collect(),
            None,
        )
        .collect(),
        ("Variable", _) => tagged(
            sorted_by_slug(rows(db, "all_variables", None).await?)
                .into_iter()
                .filter(|row| filters.matches(row, row.get("path").and_then(Value::as_str), true))
                .collect(),
            None,
        )
        .collect(),
        // Dependencies have no file path: the glob does not apply to them.
        ("Dependency", _) => tagged(
            sorted_by_slug(rows(db, "all_dependencies", None).await?)
                .into_iter()
                .filter(|row| filters.matches(row, None, false))
                .collect(),
            None,
        )
        .collect(),
        (other, _) => return Err(unknown_entity_type(other)),
    };
    Ok(matched)
}

fn unknown_entity_type(other: &str) -> String {
    format!("Unknown entity_type '{other}': expected Function, File, Type, Dependency or Variable")
}

/// Get neighbors of an AST node (callers, callees, containers, etc.).
///
/// The node's type is found first, in the query index (functions, types,
/// files) or by one variable lookup; then only that type's neighbour
/// queries run, each edge type tagged on its rows from the Rust side and
/// each query's rows in slug order. A slug that is not in the graph is a
/// not-found error.
///
/// `_depth` and `_edge_types` are reserved for future traversal filtering
/// but not yet implemented in the nanograph query layer.
pub async fn dispatch_ast_neighbors(
    db: &GraphDatabase,
    node_id: &str,
    _depth: Option<usize>,
    _edge_types: Option<&[String]>,
) -> String {
    let result = async {
        let kind = node_kind(db, node_id).await?;
        let params = serde_json::json!({ "slug": node_id });
        let mut neighbors = Vec::new();
        for nq in NEIGHBOR_QUERIES.iter().filter(|nq| nq.kind == kind) {
            for mut item in sorted_by_slug(rows(db, nq.query_name, Some(&params)).await?) {
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
        Ok::<_, String>(neighbors)
    }
    .await;
    match result {
        Ok(neighbors) => stamp(
            db,
            serde_json::json!({
                "action": "ast_neighbors",
                "node_id": node_id,
                "count": neighbors.len(),
                "neighbors": neighbors,
            }),
        ),
        Err(error) => error_result(db, "ast_neighbors", error),
    }
}

/// The type of the node `slug`, or a not-found error.
async fn node_kind(db: &GraphDatabase, slug: &str) -> Result<NodeKind, String> {
    let index = db.query_index().await?;
    if index.functions.contains_key(slug) {
        return Ok(NodeKind::Function);
    }
    if index.types.contains_key(slug) {
        return Ok(NodeKind::Type);
    }
    if index.files.contains_key(slug) {
        return Ok(NodeKind::File);
    }
    let params = serde_json::json!({ "slug": slug });
    if !rows(db, "variable_by_slug", Some(&params))
        .await?
        .is_empty()
    {
        return Ok(NodeKind::Variable);
    }
    Err(format!("Node not found: {slug}"))
}

/// Get AST codebase statistics.
///
/// Uses the GraphDatabase's built-in `stats()` method for accurate counts
/// directly from storage segments.
pub async fn dispatch_ast_stats(db: &GraphDatabase) -> String {
    stamp(db, graph_stats(db, "ast_stats"))
}
