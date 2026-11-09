//! AST Dead Code Detection Dispatch
//!
//! Detects orphan files, uncalled functions, and unreferenced types in the
//! AST code graph using nanograph `not { }` anti-join queries.
//!
//! Extracted from `ast_dispatch.rs` for file size compliance.

use crate::database::GraphDatabase;
use crate::dispatch_helpers::{error_result, stamp};
use crate::query_index::{rows, with_source, QueryIndex};
use globset::GlobMatcher;
use serde_json::Value;

use super::ast_dispatch::build_glob_matcher;

/// Why a dead-code result has no unresolved counts (KGRAPH-078).
const UNRESOLVED_UNKNOWN: &str = "The graph was not last written by ast_index (for example by a \
     bundle import), so the number of references it could not resolve is not known; run ast_index \
     to record it";

/// Detect dead code in the AST graph.
///
/// Uses nanograph `not { }` anti-join queries, which return slugs only, to
/// find:
/// - Orphan files: File nodes with no incoming Imports edges
/// - Uncalled functions: Function nodes with no incoming Calls edges
/// - Unreferenced types: Type nodes with no incoming TypeRef edges
///
/// Accepts optional `entity_type` filter ("File", "Function", "Type").
/// Accepts optional `path_pattern` glob to scope results to matching file
/// paths; an invalid glob is an error.
///
/// `unresolved` holds the unresolved reference counts per kind that the last
/// ast_index recorded (calls, imports, type refs …): an uncalled function
/// may still be called through one of them (KGRAPH-078).
///
/// Each entity type's items come in slug order: `count` items returned and
/// `total` items found (KGRAPH-077). Function and type metadata comes from
/// the query index; only the items returned get their source text. A failed
/// query is an error, never an empty list.
///
/// Applies false-positive reduction filters:
/// - Excludes test files and stubs from orphan files
/// - Excludes test functions and test file functions from uncalled functions
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
    let unresolved = match db.unresolved_record() {
        Ok(unresolved) => unresolved,
        Err(error) => return error_result(db, "ast_dead_code", error),
    };
    match dead_code(db, entity_type, limit.unwrap_or(100), path_pattern).await {
        Ok(results) => {
            let mut result = serde_json::json!({
                "action": "ast_dead_code",
                "entity_type": entity_type,
                "results": results,
                "unresolved": unresolved,
            });
            if result["unresolved"].is_null() {
                result["unresolved_unknown"] = Value::String(UNRESOLVED_UNKNOWN.to_string());
            }
            stamp(db, result)
        }
        Err(error) => error_result(db, "ast_dead_code", error),
    }
}

async fn dead_code(
    db: &GraphDatabase,
    entity_type: Option<&str>,
    max_results: usize,
    path_pattern: Option<&str>,
) -> Result<serde_json::Map<String, Value>, String> {
    let types_to_check: Vec<&str> = match entity_type {
        Some(t) => vec![t],
        None => vec!["File", "Function", "Type"],
    };
    let glob = build_glob_matcher(path_pattern)?;
    let index = db.query_index().await?;
    let is_flutter = is_flutter_project(db).await?;

    let mut all_results = serde_json::Map::new();
    for check_type in types_to_check {
        let (query_name, source_query) = match check_type {
            "File" => ("orphan_files", None),
            "Function" => ("uncalled_function_slugs", Some("function_source")),
            "Type" => ("unreferenced_type_slugs", Some("type_source")),
            other => {
                return Err(format!(
                    "Unknown entity_type '{other}': expected File, Function or Type"
                ))
            }
        };
        let mut slugs: Vec<String> = rows(db, query_name, None)
            .await?
            .iter()
            .map(|row| {
                row.get("slug")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| {
                        format!("Graph query '{query_name}' returned a row without slug: {row}")
                    })
            })
            .collect::<Result<_, _>>()?;
        slugs.sort();
        let found: Vec<(String, Value)> = slugs
            .into_iter()
            .filter_map(|slug| {
                let item = entity_row(&index, check_type, &slug)?;
                passes(&index, check_type, &slug, &item, is_flutter, glob.as_ref())
                    .then_some((slug, item))
            })
            .collect();
        let total = found.len();
        let mut items = Vec::new();
        for (slug, item) in found.into_iter().take(max_results) {
            items.push(match source_query {
                Some(source_query) => with_source(db, source_query, &slug, item).await?,
                None => item,
            });
        }
        all_results.insert(
            check_type.to_string(),
            serde_json::json!({
                "count": items.len(),
                "total": total,
                "items": items,
            }),
        );
    }
    Ok(all_results)
}

/// The index row of a dead-code candidate.
fn entity_row(index: &QueryIndex, check_type: &str, slug: &str) -> Option<Value> {
    match check_type {
        "File" => index.files.get(slug).cloned(),
        "Function" => index.functions.get(slug).cloned(),
        _ => index.types.get(slug).cloned(),
    }
}

/// The false-positive filters and the path glob.
fn passes(
    index: &QueryIndex,
    check_type: &str,
    slug: &str,
    item: &Value,
    is_flutter: bool,
    glob: Option<&GlobMatcher>,
) -> bool {
    let (path, kept) = match check_type {
        "File" => {
            let path = item.get("path").and_then(Value::as_str);
            (path, passes_file_filter(item, is_flutter))
        }
        "Function" => {
            let file = index.function_file.get(slug);
            // KGRAPH-074: a test function (test attribute, test container or
            // test file) is never dead code.
            let is_test = item.get("isTest").and_then(Value::as_bool) == Some(true)
                || index.in_test_file(file);
            let path = index.function_path(slug);
            (path, !is_test && !path.is_some_and(is_generated_dart_path))
        }
        _ => {
            let file = index.type_file.get(slug);
            let extension = item.get("typeKind").and_then(Value::as_str) == Some("extension");
            let path = index.type_path(slug);
            (
                path,
                !extension
                    && !index.in_test_file(file)
                    && !path.is_some_and(is_generated_dart_path),
            )
        }
    };
    kept && glob.is_none_or(|matcher| path.is_some_and(|p| matcher.is_match(p)))
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

/// Whether the indexed project is a Flutter project: it has a "flutter"
/// dependency.
async fn is_flutter_project(db: &GraphDatabase) -> Result<bool, String> {
    Ok(rows(db, "all_dependencies", None)
        .await?
        .iter()
        .any(|d| d.get("name").and_then(|v| v.as_str()) == Some("flutter")))
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
