// Feature: spec/features/call-chain-path-tracing-between-two-functions.feature
//
// Call Chain / Path Tracing Between Two Functions
// Tests for ast_call_chain action that finds the shortest call path(s)
// between two functions via multi-hop CALLS edge traversal.
//
// Each test populates an isolated AST graph database with known call graph data,
// then exercises the dispatch function directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_call_chain;
use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create an AST graph database with a linear call chain: A → B → C → D
async fn setup_linear_chain_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-chain.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-main-rs","path":"src/main.rs","language":"rust","lineCount":100,"isTest":false}}
{"type":"Function","data":{"slug":"src-main-rs::func_a","name":"func_a","qualifiedName":"src-main-rs::func_a","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":1,"lineEnd":10}}
{"type":"Function","data":{"slug":"src-main-rs::func_b","name":"func_b","qualifiedName":"src-main-rs::func_b","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":12,"lineEnd":20}}
{"type":"Function","data":{"slug":"src-main-rs::func_c","name":"func_c","qualifiedName":"src-main-rs::func_c","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":22,"lineEnd":30}}
{"type":"Function","data":{"slug":"src-main-rs::func_d","name":"func_d","qualifiedName":"src-main-rs::func_d","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":32,"lineEnd":40}}
{"type":"Function","data":{"slug":"src-main-rs::isolated","name":"isolated","qualifiedName":"src-main-rs::isolated","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":42,"lineEnd":50}}
{"edge":"Contains","from":"src-main-rs","to":"src-main-rs::func_a","data":{}}
{"edge":"Contains","from":"src-main-rs","to":"src-main-rs::func_b","data":{}}
{"edge":"Contains","from":"src-main-rs","to":"src-main-rs::func_c","data":{}}
{"edge":"Contains","from":"src-main-rs","to":"src-main-rs::func_d","data":{}}
{"edge":"Contains","from":"src-main-rs","to":"src-main-rs::isolated","data":{}}
{"edge":"Calls","from":"src-main-rs::func_a","to":"src-main-rs::func_b","data":{"callCount":1}}
{"edge":"Calls","from":"src-main-rs::func_b","to":"src-main-rs::func_c","data":{"callCount":1}}
{"edge":"Calls","from":"src-main-rs::func_c","to":"src-main-rs::func_d","data":{"callCount":1}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

/// Helper: create a graph with multiple paths: A→B→D (2 hops) and A→C→E→D (3 hops)
async fn setup_multi_path_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-multi-path.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-lib-rs","path":"src/lib.rs","language":"rust","lineCount":100,"isTest":false}}
{"type":"Function","data":{"slug":"src-lib-rs::entry","name":"entry","qualifiedName":"src-lib-rs::entry","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":1,"lineEnd":10}}
{"type":"Function","data":{"slug":"src-lib-rs::short_path","name":"short_path","qualifiedName":"src-lib-rs::short_path","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":12,"lineEnd":20}}
{"type":"Function","data":{"slug":"src-lib-rs::long_path_a","name":"long_path_a","qualifiedName":"src-lib-rs::long_path_a","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":22,"lineEnd":30}}
{"type":"Function","data":{"slug":"src-lib-rs::long_path_b","name":"long_path_b","qualifiedName":"src-lib-rs::long_path_b","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":32,"lineEnd":40}}
{"type":"Function","data":{"slug":"src-lib-rs::target","name":"target","qualifiedName":"src-lib-rs::target","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":42,"lineEnd":50}}
{"edge":"Contains","from":"src-lib-rs","to":"src-lib-rs::entry","data":{}}
{"edge":"Contains","from":"src-lib-rs","to":"src-lib-rs::short_path","data":{}}
{"edge":"Contains","from":"src-lib-rs","to":"src-lib-rs::long_path_a","data":{}}
{"edge":"Contains","from":"src-lib-rs","to":"src-lib-rs::long_path_b","data":{}}
{"edge":"Contains","from":"src-lib-rs","to":"src-lib-rs::target","data":{}}
{"edge":"Calls","from":"src-lib-rs::entry","to":"src-lib-rs::short_path","data":{"callCount":1}}
{"edge":"Calls","from":"src-lib-rs::short_path","to":"src-lib-rs::target","data":{"callCount":1}}
{"edge":"Calls","from":"src-lib-rs::entry","to":"src-lib-rs::long_path_a","data":{"callCount":1}}
{"edge":"Calls","from":"src-lib-rs::long_path_a","to":"src-lib-rs::long_path_b","data":{"callCount":1}}
{"edge":"Calls","from":"src-lib-rs::long_path_b","to":"src-lib-rs::target","data":{"callCount":1}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: Direct call chain between two functions
// ============================================================================
#[tokio::test]
async fn test_direct_call_chain() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_b", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a chains array containing one chain of length 1
    let chains = parsed.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(!chains.is_empty(), "Should find at least one chain");
    let first_chain = chains[0].as_object().expect("chain is object");
    let function_chain = first_chain.get("function_chain").and_then(|v| v.as_array()).expect("function_chain");
    // Chain of length 1 means 2 nodes (source + target)
    assert_eq!(function_chain.len(), 2, "Direct call should have 2 nodes in chain");

    // @step And the chain should list both functions in order from source to target
    let first_slug = function_chain[0].get("slug").and_then(|v| v.as_str()).expect("slug");
    let last_slug = function_chain[1].get("slug").and_then(|v| v.as_str()).expect("slug");
    assert_eq!(first_slug, "src-main-rs::func_a");
    assert_eq!(last_slug, "src-main-rs::func_b");
}

// ============================================================================
// Scenario: Multi-hop call chain with intermediate functions
// ============================================================================
#[tokio::test]
async fn test_multi_hop_call_chain() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed where function A calls B, B calls C, and C calls D
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "func_d"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_d", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a chains array containing a chain of length 3
    let chains = parsed.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(!chains.is_empty(), "Should find chain");
    let chain = chains[0].as_object().expect("chain is object");
    let function_chain = chain.get("function_chain").and_then(|v| v.as_array()).expect("function_chain");
    // 3-hop chain = 4 nodes: A→B→C→D
    assert_eq!(function_chain.len(), 4, "3-hop chain should have 4 nodes");

    // @step And the chain should include all intermediate functions in order A, B, C, D
    let slugs: Vec<&str> = function_chain.iter()
        .map(|n| n.get("slug").and_then(|v| v.as_str()).expect("slug"))
        .collect();
    assert_eq!(slugs, vec![
        "src-main-rs::func_a",
        "src-main-rs::func_b",
        "src-main-rs::func_c",
        "src-main-rs::func_d",
    ]);
}

// ============================================================================
// Scenario: No path exists between two unconnected functions
// ============================================================================
#[tokio::test]
async fn test_no_path_between_unconnected_functions() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed with two functions that have no call path between them
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "isolated"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::isolated", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an empty chains array
    let chains = parsed.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(chains.is_empty(), "Should find no chains for unconnected functions");

    // @step And the response should include a message indicating no path was found within the depth limit
    let message = parsed.get("message").and_then(|v| v.as_str());
    assert!(message.is_some(), "Should include a message");
    assert!(message.unwrap().contains("No call path found"), "Message should indicate no path found");
}

// ============================================================================
// Scenario: Non-existent source function slug
// ============================================================================
#[tokio::test]
async fn test_nonexistent_source_function() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed in the AST graph
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "nonexistent_function" to "func_b"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "nonexistent_function", "src-main-rs::func_b", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the source function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(error.unwrap().contains("not found"), "Error should mention 'not found'");
}

// ============================================================================
// Scenario: Non-existent target function slug
// ============================================================================
#[tokio::test]
async fn test_nonexistent_target_function() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed in the AST graph
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "nonexistent_function"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "nonexistent_function", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the target function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(error.unwrap().contains("not found"), "Error should mention 'not found'");
}

// ============================================================================
// Scenario: Max depth limits path discovery
// ============================================================================
#[tokio::test]
async fn test_max_depth_limits_discovery() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed where the shortest path between two functions is 3 hops
    let db = setup_linear_chain_db(temp_dir.path()).await;
    // func_a → func_b → func_c → func_d = 3 hops

    // @step When I request ast_call_chain with max_depth 2
    let result_shallow = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_d", Some(2),
    ).await;
    let parsed_shallow: Value = serde_json::from_str(&result_shallow).expect("valid JSON");

    // @step Then I should receive an empty chains array
    let chains_shallow = parsed_shallow.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(chains_shallow.is_empty(), "max_depth=2 should not find 3-hop path");

    // @step When I request ast_call_chain with max_depth 3
    let result_deep = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_d", Some(3),
    ).await;
    let parsed_deep: Value = serde_json::from_str(&result_deep).expect("valid JSON");

    // @step Then I should receive a chains array containing the 3-hop path
    let chains_deep = parsed_deep.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(!chains_deep.is_empty(), "max_depth=3 should find 3-hop path");
}

// ============================================================================
// Scenario: Multiple paths returned ordered by length
// ============================================================================
#[tokio::test]
async fn test_multiple_paths_ordered_by_length() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed with both a 2-hop and a 3-hop path between two functions
    let db = setup_multi_path_db(temp_dir.path()).await;
    // entry → short_path → target (2 hops)
    // entry → long_path_a → long_path_b → target (3 hops)

    // @step When I request ast_call_chain between those two functions
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-lib-rs::entry", "src-lib-rs::target", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the chains array should contain the shorter path first
    let chains = parsed.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(chains.len() >= 2, "Should find at least 2 paths");
    let first_chain_len = chains[0].get("chain_length").and_then(|v| v.as_u64()).expect("chain_length");
    let second_chain_len = chains[1].get("chain_length").and_then(|v| v.as_u64()).expect("chain_length");
    assert!(first_chain_len <= second_chain_len, "Shorter path should come first");

    // @step And results should be limited to at most 20 chains
    assert!(chains.len() <= 20, "Should not exceed 20 chains");
}

// ============================================================================
// Scenario: Chain results include function metadata and call details per hop
// ============================================================================
#[tokio::test]
async fn test_chain_results_include_function_and_call_details() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_b", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    let chains = parsed.get("chains").and_then(|v| v.as_array()).expect("chains array");
    assert!(!chains.is_empty(), "Should find at least one chain");
    let chain = chains[0].as_object().expect("chain is object");

    // @step Then each chain should contain a function_chain array with node metadata for each function
    let function_chain = chain.get("function_chain").and_then(|v| v.as_array())
        .expect("function_chain array");
    assert_eq!(function_chain.len(), 2, "Direct call: 2 functions");
    // Verify function metadata fields exist
    let first_fn = &function_chain[0];
    assert!(first_fn.get("slug").is_some(), "function_chain should have slug");
    assert!(first_fn.get("name").is_some(), "function_chain should have name");

    // @step And each chain should contain a call_details array with edge metadata for each hop
    let call_details = chain.get("call_details").and_then(|v| v.as_array())
        .expect("call_details array");
    // 1-hop chain = 1 call detail
    assert_eq!(call_details.len(), 1, "Direct call: 1 hop = 1 call_details entry");

    // @step And each chain should include a chain_length integer
    let chain_length = chain.get("chain_length").and_then(|v| v.as_u64())
        .expect("chain_length integer");
    assert_eq!(chain_length, 1, "Direct call = chain_length 1");
}

// ============================================================================
// Scenario: Successful response includes human-readable summary
// ============================================================================
#[tokio::test]
async fn test_response_includes_summary() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let db = setup_linear_chain_db(temp_dir.path()).await;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result = ast_call_chain::dispatch_ast_call_chain(
        &db, "src-main-rs::func_a", "src-main-rs::func_b", None,
    ).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the response should include a summary string describing the number of chains found
    let summary = parsed.get("summary").and_then(|v| v.as_str())
        .expect("summary string");
    assert!(summary.contains("Found"), "Summary should contain 'Found'");
    assert!(summary.contains("call chain"), "Summary should mention 'call chain'");
}
