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

use super::ast_dispatch::AST_QUERIES;
use super::database::GraphDatabase;

/// Default limit for top-N complexity queries.
const DEFAULT_LIMIT: usize = 20;

/// Dispatch a cyclomatic complexity query.
///
/// # Modes
/// - `node_id = Some(slug)`: Return complexity for a specific function
/// - `node_id = None`: Return top-N most complex functions
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
    match node_id {
        Some(slug) => dispatch_single_function(db, slug).await,
        None => {
            dispatch_top_n(
                db,
                limit.unwrap_or(DEFAULT_LIMIT),
                min_threshold,
                path_filter,
            )
            .await
        }
    }
}

/// Query complexity for a single function by slug.
async fn dispatch_single_function(db: &GraphDatabase, slug: &str) -> String {
    let rows = match db
        .query_with_source(AST_QUERIES, "all_functions", None)
        .await
    {
        Ok(Value::Array(items)) => items,
        Ok(_) => {
            return error_json("Query returned non-array result");
        }
        Err(e) => {
            return error_json(&format!("Query failed: {e}"));
        }
    };

    // Find the function matching the slug
    for row in &rows {
        let row_slug = row.get("slug").and_then(Value::as_str).unwrap_or("");
        if row_slug == slug {
            return build_single_result(row, db).await;
        }
    }

    // Not found
    error_json(&format!("No function found with slug '{slug}'"))
}

/// Build the JSON response for a single function lookup.
async fn build_single_result(row: &Value, db: &GraphDatabase) -> String {
    let slug = row.get("slug").and_then(Value::as_str).unwrap_or("");
    let name = row.get("name").and_then(Value::as_str).unwrap_or("");
    let complexity = row
        .get("cyclomaticComplexity")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let line_start = row.get("lineStart").and_then(Value::as_i64);
    let line_end = row.get("lineEnd").and_then(Value::as_i64);

    // Resolve file path via function_container query
    let path = resolve_file_path(db, slug).await;

    let mut result = serde_json::json!({
        "action": "ast_complexity",
        "slug": slug,
        "name": name,
        "cyclomaticComplexity": complexity,
        "summary": format!("{name} has cyclomatic complexity {complexity}"),
    });

    if let Some(p) = &path {
        result["path"] = Value::String(p.clone());
    }
    if let Some(ls) = line_start {
        result["lineStart"] = serde_json::json!(ls);
    }
    if let Some(le) = line_end {
        result["lineEnd"] = serde_json::json!(le);
    }

    result.to_string()
}

/// Query top-N most complex functions across the codebase.
async fn dispatch_top_n(
    db: &GraphDatabase,
    limit: usize,
    min_threshold: Option<u32>,
    path_filter: Option<&str>,
) -> String {
    let rows = match db
        .query_with_source(AST_QUERIES, "all_functions", None)
        .await
    {
        Ok(Value::Array(items)) => items,
        Ok(_) => {
            return error_json("Query returned non-array result");
        }
        Err(e) => {
            return error_json(&format!("Query failed: {e}"));
        }
    };

    // Collect functions with complexity data, resolve file paths
    let mut functions: Vec<Value> = Vec::new();

    for row in &rows {
        let complexity = row
            .get("cyclomaticComplexity")
            .and_then(Value::as_i64)
            .unwrap_or(0);

        // Skip functions below threshold
        if let Some(threshold) = min_threshold {
            if (complexity as u32) < threshold {
                continue;
            }
        }

        let slug = row.get("slug").and_then(Value::as_str).unwrap_or("");
        let name = row.get("name").and_then(Value::as_str).unwrap_or("");

        // Resolve file path
        let path = resolve_file_path(db, slug).await;

        // Apply path filter
        if let Some(filter) = path_filter {
            if let Some(ref p) = path {
                if !p.contains(filter) {
                    continue;
                }
            } else {
                continue;
            }
        }

        let mut entry = serde_json::json!({
            "slug": slug,
            "name": name,
            "cyclomaticComplexity": complexity,
        });

        if let Some(ref p) = path {
            entry["path"] = Value::String(p.clone());
        }
        if let Some(ls) = row.get("lineStart").and_then(Value::as_i64) {
            entry["lineStart"] = serde_json::json!(ls);
        }
        if let Some(le) = row.get("lineEnd").and_then(Value::as_i64) {
            entry["lineEnd"] = serde_json::json!(le);
        }

        functions.push(entry);
    }

    // Sort by complexity descending
    functions.sort_by(|a, b| {
        let ac = a
            .get("cyclomaticComplexity")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let bc = b
            .get("cyclomaticComplexity")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        bc.cmp(&ac)
    });

    // Apply limit
    functions.truncate(limit);

    let count = functions.len();
    serde_json::json!({
        "action": "ast_complexity",
        "results": functions,
        "count": count,
        "summary": format!("Top {count} most complex functions"),
    })
    .to_string()
}

/// Resolve a function's file path via the function_container query.
async fn resolve_file_path(db: &GraphDatabase, fn_slug: &str) -> Option<String> {
    let params = serde_json::json!({ "slug": fn_slug });
    if let Ok(Value::Array(container_rows)) = db
        .query_with_source(AST_QUERIES, "function_container", Some(&params))
        .await
    {
        if let Some(first) = container_rows.first() {
            return first.get("path").and_then(Value::as_str).map(String::from);
        }
    }
    None
}

/// Build a standard error JSON response.
fn error_json(message: &str) -> String {
    serde_json::json!({
        "action": "ast_complexity",
        "error": message,
    })
    .to_string()
}
