//! Cyclomatic Complexity Query Dispatch
//!
//! Queries the AST graph for function complexity data.
//! Supports two modes:
//! - **Single function**: Given a `node_id`, returns that function's complexity
//! - **Top-N**: Returns the most complex functions sorted by complexity DESC
//!
//! CGC equivalent: `code_finder.get_cyclomatic_complexity()` and
//! `code_finder.find_most_complex_functions()`.

use serde_json::Value;

use super::database::GraphDatabase;
use super::dispatch_helpers::{error_result, stamp};
use super::query_index::QueryIndex;

/// Default limit for top-N complexity queries.
const DEFAULT_LIMIT: usize = 20;

const ACTION: &str = "ast_complexity";

/// Dispatch a cyclomatic complexity query.
///
/// Answered from the graph's query index (KGRAPH-077): no query runs per
/// function, and a failed index query is an error.
///
/// # Modes
/// - `node_id = Some(slug)`: Return complexity for a specific function
/// - `node_id = None`: Return top-N most complex functions, by complexity
///   descending and then by slug, with `total` counting every function
///   that passed the filters
///
/// # Parameters
/// - `db`: The AST graph database
/// - `node_id`: Optional function slug for single-function mode
/// - `limit`: Maximum results to return (default: 20)
/// - `min_threshold`: Only return functions with complexity >= this value
/// - `path_filter`: Only include functions from files matching this substring
pub async fn dispatch_ast_complexity(
    db: &GraphDatabase,
    node_id: Option<&str>,
    limit: Option<usize>,
    min_threshold: Option<u32>,
    path_filter: Option<&str>,
) -> String {
    let index = match db.query_index().await {
        Ok(index) => index,
        Err(error) => return error_result(db, ACTION, error),
    };
    let result = match node_id {
        Some(slug) => single_function(&index, slug),
        None => Ok(top_n(
            &index,
            limit.unwrap_or(DEFAULT_LIMIT),
            min_threshold,
            path_filter,
        )),
    };
    match result {
        Ok(result) => stamp(db, result),
        Err(error) => error_result(db, ACTION, error),
    }
}

/// Complexity for a single function by slug.
fn single_function(index: &QueryIndex, slug: &str) -> Result<Value, String> {
    let row = index
        .functions
        .get(slug)
        .ok_or_else(|| format!("No function found with slug '{slug}'"))?;
    let name = row.get("name").and_then(Value::as_str).unwrap_or("");
    let complexity = row
        .get("cyclomaticComplexity")
        .and_then(Value::as_i64)
        .unwrap_or(1);

    let mut result = serde_json::json!({
        "action": ACTION,
        "slug": slug,
        "name": name,
        "cyclomaticComplexity": complexity,
        "summary": format!("{name} has cyclomatic complexity {complexity}"),
    });
    if let Some(path) = index.function_path(slug) {
        result["path"] = Value::String(path.to_string());
    }
    if let Some(ls) = row.get("lineStart").and_then(Value::as_i64) {
        result["lineStart"] = serde_json::json!(ls);
    }
    if let Some(le) = row.get("lineEnd").and_then(Value::as_i64) {
        result["lineEnd"] = serde_json::json!(le);
    }
    Ok(result)
}

/// The top-N most complex functions across the codebase.
fn top_n(
    index: &QueryIndex,
    limit: usize,
    min_threshold: Option<u32>,
    path_filter: Option<&str>,
) -> Value {
    let mut functions: Vec<(i64, &str, Value)> = Vec::new();
    for (slug, row) in &index.functions {
        let complexity = row
            .get("cyclomaticComplexity")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if min_threshold.is_some_and(|threshold| complexity < i64::from(threshold)) {
            continue;
        }
        let path = index.function_path(slug);
        if let Some(filter) = path_filter {
            if !path.is_some_and(|p| p.contains(filter)) {
                continue;
            }
        }
        let name = row.get("name").and_then(Value::as_str).unwrap_or("");
        let mut entry = serde_json::json!({
            "slug": slug,
            "name": name,
            "cyclomaticComplexity": complexity,
        });
        if let Some(p) = path {
            entry["path"] = Value::String(p.to_string());
        }
        if let Some(ls) = row.get("lineStart").and_then(Value::as_i64) {
            entry["lineStart"] = serde_json::json!(ls);
        }
        if let Some(le) = row.get("lineEnd").and_then(Value::as_i64) {
            entry["lineEnd"] = serde_json::json!(le);
        }
        functions.push((complexity, slug, entry));
    }

    // Complexity descending, then slug ascending.
    functions.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let total = functions.len();
    let results: Vec<Value> = functions
        .into_iter()
        .take(limit)
        .map(|(_, _, entry)| entry)
        .collect();
    let count = results.len();
    serde_json::json!({
        "action": ACTION,
        "results": results,
        "count": count,
        "total": total,
        "summary": format!("Top {count} most complex functions of {total}"),
    })
}
