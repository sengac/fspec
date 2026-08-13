// Feature: spec/features/cyclomatic-complexity-analysis.feature
//
// Cyclomatic Complexity Analysis
// Tests for ast_complexity action that queries function complexity scores
// from the AST graph, and for the complexity calculation module that
// computes cyclomatic complexity during extraction.
//
// Integration tests populate an isolated AST graph with Function nodes
// that have cyclomaticComplexity values, then exercise the dispatch
// function directly. Unit tests verify the complexity calculator.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_complexity;
use codelet_napi::graph::ast_pipeline::complexity;
use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema (must include cyclomaticComplexity on Function).
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create a graph with functions of varying complexity.
///
/// Functions:
///   - `complex_handler` — complexity 12 (highest)
///   - `process_data`    — complexity 8
///   - `validate_input`  — complexity 6
///   - `simple_getter`   — complexity 1 (lowest)
///   - `medium_logic`    — complexity 4
async fn setup_complexity_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-complexity.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-handler-ts","path":"src/handler.ts","language":"typescript","lineCount":200,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-ts","path":"src/utils.ts","language":"typescript","lineCount":50,"isTest":false}}
{"type":"Function","data":{"slug":"src-handler-ts::complex_handler","name":"complex_handler","qualifiedName":"src-handler-ts::complex_handler","isAsync":true,"isPublic":true,"paramCount":3,"lineStart":1,"lineEnd":50,"cyclomaticComplexity":12}}
{"type":"Function","data":{"slug":"src-handler-ts::process_data","name":"process_data","qualifiedName":"src-handler-ts::process_data","isAsync":false,"isPublic":true,"paramCount":2,"lineStart":52,"lineEnd":80,"cyclomaticComplexity":8}}
{"type":"Function","data":{"slug":"src-handler-ts::validate_input","name":"validate_input","qualifiedName":"src-handler-ts::validate_input","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":82,"lineEnd":100,"cyclomaticComplexity":6}}
{"type":"Function","data":{"slug":"src-utils-ts::simple_getter","name":"simple_getter","qualifiedName":"src-utils-ts::simple_getter","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":1,"lineEnd":3,"cyclomaticComplexity":1}}
{"type":"Function","data":{"slug":"src-utils-ts::medium_logic","name":"medium_logic","qualifiedName":"src-utils-ts::medium_logic","isAsync":false,"isPublic":false,"paramCount":2,"lineStart":5,"lineEnd":25,"cyclomaticComplexity":4}}
{"edge":"Contains","from":"src-handler-ts","to":"src-handler-ts::complex_handler","data":{}}
{"edge":"Contains","from":"src-handler-ts","to":"src-handler-ts::process_data","data":{}}
{"edge":"Contains","from":"src-handler-ts","to":"src-handler-ts::validate_input","data":{}}
{"edge":"Contains","from":"src-utils-ts","to":"src-utils-ts::simple_getter","data":{}}
{"edge":"Contains","from":"src-utils-ts","to":"src-utils-ts::medium_logic","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: Find top N most complex functions in a codebase
// ============================================================================
#[tokio::test]
async fn test_find_top_n_most_complex_functions() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed with cyclomatic complexity calculated for all functions
    // (graph loaded above with 5 functions of varying complexity)

    // @step When I request ast_complexity with limit 10
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,     // no specific function — top-N mode
        Some(10), // limit
        None,     // no min_threshold
        None,     // no path filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive a list of 10 functions sorted by complexity descending
    assert_eq!(result["action"], "ast_complexity");
    let results = result["results"].as_array().expect("results array");
    // We have 5 functions, so we get all 5 (limit 10 > 5)
    assert_eq!(results.len(), 5);
    // Verify descending order
    assert_eq!(results[0]["name"], "complex_handler");
    assert_eq!(results[0]["cyclomaticComplexity"], 12);
    assert_eq!(results[1]["name"], "process_data");
    assert_eq!(results[1]["cyclomaticComplexity"], 8);
    assert_eq!(results[2]["name"], "validate_input");
    assert_eq!(results[2]["cyclomaticComplexity"], 6);

    // @step And each result should include function name, file path, line numbers, and complexity score
    let first = &results[0];
    assert!(first.get("name").is_some());
    assert!(first.get("path").is_some());
    assert!(first.get("lineStart").is_some());
    assert!(first.get("lineEnd").is_some());
    assert!(first.get("cyclomaticComplexity").is_some());
}

// ============================================================================
// Scenario: Find top N with limit smaller than total
// ============================================================================
#[tokio::test]
async fn test_find_top_n_with_limit() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(3), // only top 3
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["cyclomaticComplexity"], 12);
    assert_eq!(results[1]["cyclomaticComplexity"], 8);
    assert_eq!(results[2]["cyclomaticComplexity"], 6);
}

// ============================================================================
// Scenario: Find top N with min_threshold filter
// ============================================================================
#[tokio::test]
async fn test_find_top_n_with_threshold() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(20),
        Some(5), // only complexity >= 5
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3); // complex_handler(12), process_data(8), validate_input(6)
    for r in results {
        assert!(r["cyclomaticComplexity"].as_u64().unwrap() >= 5);
    }
}

// ============================================================================
// Scenario: Query complexity of a specific function
// ============================================================================
#[tokio::test]
async fn test_query_specific_function_complexity() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed with cyclomatic complexity calculated
    // (graph loaded above)

    // @step When I request ast_complexity for a specific function slug
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        Some("src-handler-ts::process_data"),
        None,
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive that function's cyclomatic complexity score
    assert_eq!(result["action"], "ast_complexity");
    assert_eq!(result["cyclomaticComplexity"], 8);

    // @step And the response should include the function name, file path, and line numbers
    assert_eq!(result["name"], "process_data");
    assert!(result.get("path").is_some());
    assert!(result.get("lineStart").is_some());
    assert!(result.get("lineEnd").is_some());
}

// ============================================================================
// Scenario: Simple function has complexity 1
// ============================================================================
#[tokio::test]
async fn test_simple_function_complexity_one() {
    // @step Given I have indexed a function with no branches or decision points
    let source = "function getter() { return 42; }";

    // @step When I query its cyclomatic complexity
    let result = complexity::calculate(source, "typescript");

    // @step Then the complexity score should be 1
    assert_eq!(result, 1);
}

// ============================================================================
// Scenario: Function with multiple branches has correct complexity
// ============================================================================
#[tokio::test]
async fn test_function_with_branches_correct_complexity() {
    // @step Given I have indexed a function with 5 if/else branches
    let source = r#"function classify(x) {
        if (x > 100) {
            return "huge";
        } else if (x > 50) {
            return "big";
        } else if (x > 20) {
            return "medium";
        } else if (x > 5) {
            return "small";
        } else if (x > 0) {
            return "tiny";
        }
        return "zero";
    }"#;

    // @step When I query its cyclomatic complexity
    let result = complexity::calculate(source, "typescript");

    // @step Then the complexity score should be 6
    assert_eq!(result, 6, "1 base + 5 if/else if branches = 6");
}

// ============================================================================
// Scenario: Complexity is populated during ast_index
// ============================================================================
#[tokio::test]
async fn test_complexity_populated_during_extraction() {
    use codelet_napi::graph::ast_pipeline::ast_ts_extractor;
    use std::collections::HashSet;

    // @step Given I have a codebase that has not been indexed
    let source = r#"
export function handleRequest(req: Request): Response {
    if (req.method === "GET") {
        return getHandler(req);
    } else if (req.method === "POST") {
        return postHandler(req);
    }
    for (const header of req.headers) {
        if (header.key === "auth") {
            return unauthorized();
        }
    }
    return notFound();
}

export function simpleReturn(): string {
    return "hello";
}
"#;

    // @step When I run ast_index on the codebase
    let entities = ast_ts_extractor::extract_typescript(source, "src/server.ts", &HashSet::new())
        .expect("extraction");

    // @step Then the Function nodes in the graph should have cyclomaticComplexity values
    let function_nodes: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, codelet_napi::graph::graph_entities::GraphEntity::Node { node_type, .. } if node_type == "Function"))
        .collect();
    assert!(
        function_nodes.len() >= 2,
        "Should extract at least 2 functions"
    );

    // Find the complexity values
    let mut found_complex = false;
    let mut found_simple = false;
    for entity in &function_nodes {
        if let codelet_napi::graph::graph_entities::GraphEntity::Node { properties, .. } = entity {
            let name = properties
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let complexity = properties
                .get("cyclomaticComplexity")
                .and_then(|v| v.as_i64());
            if name == "handleRequest" {
                // @step And functions with decision points should have complexity greater than 1
                assert!(complexity.is_some(), "handleRequest should have complexity");
                assert!(
                    complexity.unwrap() > 1,
                    "handleRequest has branches, complexity should be > 1, got {:?}",
                    complexity
                );
                found_complex = true;
            } else if name == "simpleReturn" {
                assert!(complexity.is_some(), "simpleReturn should have complexity");
                assert_eq!(
                    complexity.unwrap(),
                    1,
                    "simpleReturn has no branches, complexity should be 1"
                );
                found_simple = true;
            }
        }
    }
    assert!(found_complex, "Should find handleRequest function");
    assert!(found_simple, "Should find simpleReturn function");
}

// ============================================================================
// Scenario: Non-existent function returns error
// ============================================================================
#[tokio::test]
async fn test_nonexistent_function_returns_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed in the AST graph
    // (graph loaded above)

    // @step When I request ast_complexity for a non-existent function slug
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        Some("nonexistent::function_xyz"),
        None,
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive an error indicating the function was not found
    assert_eq!(result["action"], "ast_complexity");
    assert!(
        result.get("error").is_some(),
        "Should have error field for non-existent function"
    );
    let error_msg = result["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("not found") || error_msg.contains("No function"),
        "Error should mention function not found, got: {error_msg}"
    );
}

// ============================================================================
// Unit tests: Complexity calculator across languages
// ============================================================================

#[test]
fn test_complexity_python() {
    let source = r#"
def process(data):
    if data is None:
        return None
    for item in data:
        if item > 0 and item < 100:
            yield item
        elif item < 0:
            raise ValueError("negative")
    while len(data) > 0:
        data.pop()
"#;
    // 1 base + if + for + if + and + elif + while = 7
    let result = complexity::calculate(source, "python");
    assert_eq!(
        result, 7,
        "Python: 1 + if + for + (if + and) + elif + while"
    );
}

#[test]
fn test_complexity_rust() {
    let source = r#"
fn dispatch(action: Action) -> Result<()> {
    if action.is_valid() {
        for item in action.items() {
            match item.kind {
                Kind::A => handle_a(),
                Kind::B => handle_b(),
                Kind::C => handle_c(),
            }
        }
    }
    while let Some(pending) = queue.pop() {
        if pending.ready() && pending.valid() {
            process(pending);
        }
    }
    Ok(())
}
"#;
    // 1 base + if + for + 3 match arms (=>) + while + if + && = 9
    let result = complexity::calculate(source, "rust");
    assert_eq!(
        result, 9,
        "Rust: 1 + if + for + 3 match arms + while + if + &&"
    );
}

#[test]
fn test_complexity_go() {
    let source = r#"
func handleRequest(w http.ResponseWriter, r *http.Request) {
    if r.Method == "GET" {
        getHandler(w, r)
    }
    for _, header := range r.Header {
        if header == "auth" || header == "token" {
            authenticate(w, r)
        }
    }
    switch r.URL.Path {
    case "/api":
        apiHandler(w, r)
    case "/health":
        healthHandler(w, r)
    }
}
"#;
    // 1 base + if + for + if + || + 2 cases = 7
    let result = complexity::calculate(source, "go");
    assert_eq!(result, 7, "Go: 1 + if + for + if + || + 2 cases");
}

#[test]
fn test_complexity_ruby() {
    let source = r#"
def process(input)
    if input.nil?
        return nil
    unless input.empty?
        input.each do |item|
            case item.type
            when :a
                handle_a
            when :b
                handle_b
            end
            rescue StandardError => e
                log_error(e)
        end
    end
end
"#;
    // 1 base + if + unless + for (each) + 2 when + rescue = 7
    let result = complexity::calculate(source, "ruby");
    assert_eq!(result, 7, "Ruby: 1 + if + unless + for + 2 when + rescue");
}

#[test]
fn test_complexity_empty_function() {
    let source = "function noop() {}";
    let result = complexity::calculate(source, "typescript");
    assert_eq!(result, 1, "Empty function should have complexity 1");
}

#[test]
fn test_complexity_operators_only() {
    let source = r#"
function check(a, b, c, d) {
    return a && b || c && d;
}
"#;
    // 1 base + && + || + && = 4
    let result = complexity::calculate(source, "typescript");
    assert_eq!(result, 4, "TS: 1 + 3 logical operators");
}

// ============================================================================
// Path-filtered top-N query
// ============================================================================
#[tokio::test]
async fn test_find_top_n_with_path_filter() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_complexity_db(temp_dir.path()).await;

    // Filter to only src/handler.ts functions
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(20),
        None,
        Some("src/handler"), // path glob filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    // Only 3 functions in src/handler.ts
    assert_eq!(results.len(), 3);
    for r in results {
        let path = r["path"].as_str().unwrap_or("");
        assert!(
            path.contains("handler"),
            "All results should be from handler path, got: {path}"
        );
    }
}
