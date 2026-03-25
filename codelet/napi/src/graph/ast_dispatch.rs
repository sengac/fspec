//! AST Graph Dispatch Functions
//!
//! Implements query dispatch logic for AST-specific GraphSearch actions.
//! Routes queries to the AST code graph database (dual-graph architecture).
//!
//! Each function takes a `GraphDatabase` reference and returns a JSON string.

use crate::graph::database::GraphDatabase;
use crate::graph::dispatch_helpers::{format_graph_stats, matches_fields, AST_SEARCHABLE_FIELDS};
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

/// Build a compiled glob matcher from an optional path pattern.
///
/// Returns `None` if no pattern is provided or if the pattern is invalid.
/// Used by `dispatch_ast_search` and `dispatch_ast_dead_code` to filter
/// results by file path.
fn build_glob_matcher(path_pattern: Option<&str>) -> Option<GlobMatcher> {
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
fn matches_path_glob(item: &Value, matcher: &GlobMatcher, path_field: &str) -> bool {
    item.get(path_field)
        .and_then(|v| v.as_str())
        .is_some_and(|path| matcher.is_match(path))
}

/// Look up all file paths from the graph, returning a slug → path map.
///
/// Used to resolve Function/Type slugs (which contain the file slug prefix)
/// back to the actual file path for glob matching.
async fn build_file_path_index(db: &GraphDatabase) -> std::collections::HashMap<String, String> {
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
fn matches_entity_path_glob(
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
pub async fn dispatch_ast_search(
    db: &GraphDatabase,
    query: &str,
    entity_type: Option<&str>,
    limit: Option<usize>,
    path_pattern: Option<&str>,
) -> String {
    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(20);
    let mut results = Vec::new();

    let glob_matcher = build_glob_matcher(path_pattern);

    // Build file path index if we need to resolve Function/Type paths
    let file_path_index = if glob_matcher.is_some() {
        build_file_path_index(db).await
    } else {
        std::collections::HashMap::new()
    };

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
                    if !matches_fields(&item, &query_lower, AST_SEARCHABLE_FIELDS) {
                        continue;
                    }
                    // Apply glob filter if provided
                    if let Some(ref matcher) = glob_matcher {
                        let matches = match search_type {
                            "File" => matches_path_glob(&item, matcher, "path"),
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

/// Index (or re-index) the project codebase into the AST graph.
///
/// Walks the project directory, extracts functions/types/imports via ast-grep,
/// extracts dependencies from Cargo.toml/package.json, and batch-loads
/// everything into the AST code graph. Idempotent — uses nanograph upsert
/// semantics so repeated calls are safe.
///
/// When `custom_path` is provided, indexes only that directory with
/// `.gitignore` disabled. Falls back to `cwd` when `None`.
pub async fn dispatch_ast_index(custom_path: Option<&str>) -> String {
    let (project_root, respect_gitignore) = if let Some(p) = custom_path {
        let path = std::path::Path::new(p);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path),
                Err(e) => {
                    return serde_json::json!({
                        "action": "ast_index",
                        "error": format!("Failed to get current directory: {e}"),
                    })
                    .to_string();
                }
            }
        };
        if !resolved.is_dir() {
            return serde_json::json!({
                "action": "ast_index",
                "error": format!("Path is not a directory: {}", resolved.display()),
            })
            .to_string();
        }
        (resolved, false)
    } else {
        match std::env::current_dir() {
            Ok(p) => (p, true),
            Err(e) => {
                return serde_json::json!({
                    "action": "ast_index",
                    "error": format!("Failed to get current directory: {e}"),
                })
                .to_string();
            }
        }
    };

    let db = match super::registry::get_graph(super::registry::AST_CODE_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            return serde_json::json!({
                "action": "ast_index",
                "error": e,
            })
            .to_string();
        }
    };

    // Walk codebase and extract AST entities
    let mut all_entities = match super::ast_pipeline::walk_and_extract(&project_root, respect_gitignore) {
        Ok(entities) => entities,
        Err(e) => {
            return serde_json::json!({
                "action": "ast_index",
                "error": format!("AST extraction failed: {e}"),
            })
            .to_string();
        }
    };

    // Extract dependencies from Cargo.toml
    match super::ast_pipeline::cargo_dep_extractor::extract_cargo_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Cargo dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from package.json
    match super::ast_pipeline::npm_dep_extractor::extract_npm_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("NPM dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from requirements.txt / pyproject.toml
    match super::ast_pipeline::pip_dep_extractor::extract_python_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Python dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from go.mod
    match super::ast_pipeline::gomod_dep_extractor::extract_go_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Go dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from pom.xml / build.gradle
    match super::ast_pipeline::java_dep_extractor::extract_java_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Java dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from composer.json
    match super::ast_pipeline::composer_dep_extractor::extract_composer_dependencies(&project_root)
    {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Composer dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from Gemfile
    match super::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Gemfile dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from .csproj files
    match super::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("C# dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from build.sbt
    match super::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("SBT dependency extraction failed (non-fatal): {e}");
        }
    }

    // Extract dependencies from Package.swift
    match super::ast_pipeline::swift_dep_extractor::extract_swift_dependencies(&project_root) {
        Ok(dep_entities) => all_entities.extend(dep_entities),
        Err(e) => {
            tracing::warn!("Swift dependency extraction failed (non-fatal): {e}");
        }
    }

    // Deduplicate after merging dep-extractor results with AST entities.
    // walk_and_extract already deduplicates internally, but dep extractors
    // may emit File nodes that overlap (e.g. Package.swift is both a Swift
    // source file and the SPM manifest), so we run a final pass.
    let all_entities = super::ast_pipeline::deduplicate_entities(all_entities);

    let entity_count = all_entities.len();
    if entity_count == 0 {
        return serde_json::json!({
            "action": "ast_index",
            "entities_loaded": 0,
            "message": "No source files found to index",
        })
        .to_string();
    }

    // Batch-load into graph (overwrite to remove stale entities from prior index)
    match db.load_entities_overwrite(&all_entities).await {
        Ok(loaded) => {
            tracing::info!(loaded, "AST index complete — entities loaded into graph");
            // Return fresh stats after indexing
            let stats = format_graph_stats(&db, "ast_index");
            // Merge stats with load count
            if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&stats) {
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert(
                        "entities_loaded".to_string(),
                        serde_json::Value::Number(loaded.into()),
                    );
                }
                parsed.to_string()
            } else {
                stats
            }
        }
        Err(e) => {
            serde_json::json!({
                "action": "ast_index",
                "error": format!("Failed to load entities into graph: {e}"),
            })
            .to_string()
        }
    }
}

/// Detect dead code in the AST graph.
///
/// Uses nanograph `not { }` anti-join queries to find:
/// - Orphan files: File nodes with no incoming Imports edges
/// - Uncalled functions: Function nodes with no incoming Calls edges
/// - Unreferenced types: Type nodes with no incoming TypeRef edges
///
/// Accepts optional `entity_type` filter ("File", "Function", "Type").
/// Accepts optional `path_pattern` glob to scope results to matching file paths.
/// Excludes test files and stub File nodes (no language) by default.
pub async fn dispatch_ast_dead_code(
    db: &GraphDatabase,
    entity_type: Option<&str>,
    limit: Option<usize>,
    path_pattern: Option<&str>,
) -> String {
    let max_results = limit.unwrap_or(100);
    let types_to_check: Vec<&str> = match entity_type {
        Some(t) => vec![t],
        None => vec!["File", "Function", "Type"],
    };

    let glob_matcher = build_glob_matcher(path_pattern);

    // Build file path index if we need to resolve Function/Type paths
    let file_path_index = if glob_matcher.is_some() {
        build_file_path_index(db).await
    } else {
        std::collections::HashMap::new()
    };

    let mut all_results = serde_json::Map::new();

    for check_type in &types_to_check {
        let query_name = match *check_type {
            "File" => "orphan_files",
            "Function" => "uncalled_functions",
            "Type" => "unreferenced_types",
            _ => continue,
        };

        match db.query_with_source(AST_QUERIES, query_name, None).await {
            Ok(Value::Array(items)) => {
                let filtered: Vec<Value> = items
                    .into_iter()
                    .filter(|item| {
                        // For files: exclude test files and stubs (no language = external import)
                        if *check_type == "File" {
                            let is_test = item
                                .get("isTest")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let has_language =
                                item.get("language").and_then(|v| v.as_str()).is_some();
                            if is_test || !has_language {
                                return false;
                            }
                        }
                        // Apply glob filter if provided
                        if let Some(ref matcher) = glob_matcher {
                            match *check_type {
                                "File" => matches_path_glob(item, matcher, "path"),
                                "Function" | "Type" => {
                                    matches_entity_path_glob(item, matcher, &file_path_index)
                                }
                                _ => true,
                            }
                        } else {
                            true
                        }
                    })
                    .take(max_results)
                    .collect();

                all_results.insert(
                    check_type.to_string(),
                    serde_json::json!({
                        "count": filtered.len(),
                        "items": filtered,
                    }),
                );
            }
            Ok(_) => {
                all_results.insert(
                    check_type.to_string(),
                    serde_json::json!({"count": 0, "items": []}),
                );
            }
            Err(e) => {
                warn!(query_name, error = %e, "Dead code query failed");
                all_results.insert(
                    check_type.to_string(),
                    serde_json::json!({"error": e.to_string()}),
                );
            }
        }
    }

    serde_json::json!({
        "action": "ast_dead_code",
        "entity_type": entity_type,
        "results": all_results,
    })
    .to_string()
}
