//! AST Dead Code Detection Dispatch
//!
//! Detects orphan files, uncalled functions, and unreferenced types in the
//! AST code graph using nanograph `not { }` anti-join queries.
//!
//! Extracted from `ast_dispatch.rs` for file size compliance.

use crate::database::GraphDatabase;
use serde_json::Value;
use tracing::warn;

use super::ast_dispatch::{
    build_file_path_index, build_glob_matcher, matches_entity_path_glob, matches_path_glob,
    AST_QUERIES,
};

/// Detect dead code in the AST graph.
///
/// Uses nanograph `not { }` anti-join queries to find:
/// - Orphan files: File nodes with no incoming Imports edges
/// - Uncalled functions: Function nodes with no incoming Calls edges
/// - Unreferenced types: Type nodes with no incoming TypeRef edges
///
/// Accepts optional `entity_type` filter ("File", "Function", "Type").
/// Accepts optional `path_pattern` glob to scope results to matching file paths.
///
/// Applies false-positive reduction filters:
/// - Excludes test files and stubs from orphan files
/// - Excludes test file functions from uncalled functions
/// - Excludes generated file entities (.g.dart, .freezed.dart)
/// - Excludes Flutter platform directories for Flutter projects
/// - Excludes main.dart entry points from orphan files
/// - Excludes extension types (typeKind="extension") from unreferenced types
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

    // Build file path index — needed for resolving Function/Type → file path
    let file_path_index = build_file_path_index(db).await;

    // Detect Flutter project by checking for "flutter" dependency
    let is_flutter = is_flutter_project(db).await;

    // Build file isTest index for filtering functions/types from test files
    let file_test_index = build_file_test_index(db).await;

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
                        // Apply entity-specific filters
                        if !passes_entity_filter(
                            item,
                            check_type,
                            is_flutter,
                            &file_path_index,
                            &file_test_index,
                        ) {
                            return false;
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

/// Apply entity-specific false-positive filters for dead code detection.
fn passes_entity_filter(
    item: &Value,
    check_type: &str,
    is_flutter: bool,
    file_path_index: &std::collections::HashMap<String, String>,
    file_test_index: &std::collections::HashMap<String, bool>,
) -> bool {
    match check_type {
        "File" => passes_file_filter(item, is_flutter),
        "Function" => passes_function_filter(item, file_path_index, file_test_index),
        "Type" => passes_type_filter(item, file_path_index, file_test_index),
        _ => true,
    }
}

/// File-level dead code filters.
fn passes_file_filter(item: &Value, is_flutter: bool) -> bool {
    let is_test = item
        .get("isTest")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_language = item.get("language").and_then(|v| v.as_str()).is_some();
    if is_test || !has_language {
        return false;
    }

    if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
        if path.ends_with("main.dart") {
            return false;
        }
        if is_generated_dart_path(path) {
            return false;
        }
        if is_flutter && is_flutter_platform_path(path) {
            return false;
        }
    }

    true
}

/// Function-level dead code filters.
fn passes_function_filter(
    item: &Value,
    file_path_index: &std::collections::HashMap<String, String>,
    file_test_index: &std::collections::HashMap<String, bool>,
) -> bool {
    if let Some(slug) = item.get("slug").and_then(|v| v.as_str()) {
        if let Some(file_slug) = slug.split("::").next() {
            if file_test_index.get(file_slug).copied().unwrap_or(false) {
                return false;
            }
            if let Some(file_path) = file_path_index.get(file_slug) {
                if is_generated_dart_path(file_path) {
                    return false;
                }
            }
        }
    }
    true
}

/// Type-level dead code filters.
fn passes_type_filter(
    item: &Value,
    file_path_index: &std::collections::HashMap<String, String>,
    file_test_index: &std::collections::HashMap<String, bool>,
) -> bool {
    if let Some(type_kind) = item.get("typeKind").and_then(|v| v.as_str()) {
        if type_kind == "extension" {
            return false;
        }
    }

    if let Some(slug) = item.get("slug").and_then(|v| v.as_str()) {
        if let Some(file_slug) = slug.split("::").next() {
            if file_test_index.get(file_slug).copied().unwrap_or(false) {
                return false;
            }
            if let Some(file_path) = file_path_index.get(file_slug) {
                if is_generated_dart_path(file_path) {
                    return false;
                }
            }
        }
    }
    true
}

/// Check if the indexed project is a Flutter project by looking for "flutter" dependency.
async fn is_flutter_project(db: &GraphDatabase) -> bool {
    if let Ok(Value::Array(deps)) = db
        .query_with_source(AST_QUERIES, "all_dependencies", None)
        .await
    {
        return deps
            .iter()
            .any(|d| d.get("name").and_then(|v| v.as_str()) == Some("flutter"));
    }
    false
}

/// Build a map of file_slug → isTest for filtering entities from test files.
async fn build_file_test_index(db: &GraphDatabase) -> std::collections::HashMap<String, bool> {
    let mut index = std::collections::HashMap::new();
    if let Ok(Value::Array(files)) = db.query_with_source(AST_QUERIES, "all_files", None).await {
        for file in files {
            if let Some(slug) = file.get("slug").and_then(|v| v.as_str()) {
                let is_test = file
                    .get("isTest")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                index.insert(slug.to_string(), is_test);
            }
        }
    }
    index
}

/// Check if a file path is a Dart generated file (.g.dart or .freezed.dart).
fn is_generated_dart_path(path: &str) -> bool {
    path.ends_with(".g.dart") || path.ends_with(".freezed.dart")
}

/// Check if a file path belongs to a Flutter platform directory.
fn is_flutter_platform_path(path: &str) -> bool {
    const FLUTTER_PLATFORM_DIRS: &[&str] = &["ios/", "android/", "macos/", "linux/", "windows/"];
    FLUTTER_PLATFORM_DIRS
        .iter()
        .any(|dir| path.starts_with(dir))
}
