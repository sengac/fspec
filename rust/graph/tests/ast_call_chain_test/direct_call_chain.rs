// Feature: spec/features/call-chain-path-tracing-between-two-functions.feature
use super::*;

// ============================================================================
// Scenario: Direct call chain between two functions
// ============================================================================
#[tokio::test]
async fn test_direct_call_chain() {
    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_b"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a chains array containing one chain of length 1
    let chains = parsed
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(!chains.is_empty(), "Should find at least one chain");
    let first_chain = chains[0].as_object().expect("chain is object");
    let function_chain = first_chain
        .get("function_chain")
        .and_then(|v| v.as_array())
        .expect("function_chain");
    // Chain of length 1 means 2 nodes (source + target)
    assert_eq!(
        function_chain.len(),
        2,
        "Direct call should have 2 nodes in chain"
    );

    // @step And the chain should list both functions in order from source to target
    let first_slug = function_chain[0]
        .get("slug")
        .and_then(|v| v.as_str())
        .expect("slug");
    let last_slug = function_chain[1]
        .get("slug")
        .and_then(|v| v.as_str())
        .expect("slug");
    assert_eq!(first_slug, fx.s("func_a"));
    assert_eq!(last_slug, fx.s("func_b"));
}

// ============================================================================
// Scenario: Multi-hop call chain with intermediate functions
// ============================================================================
#[tokio::test]
async fn test_multi_hop_call_chain() {
    // @step Given I have a codebase indexed where function A calls B, B calls C, and C calls D
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "func_d"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_d"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a chains array containing a chain of length 3
    let chains = parsed
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(!chains.is_empty(), "Should find chain");
    let chain = chains[0].as_object().expect("chain is object");
    let function_chain = chain
        .get("function_chain")
        .and_then(|v| v.as_array())
        .expect("function_chain");
    // 3-hop chain = 4 nodes: A→B→C→D
    assert_eq!(function_chain.len(), 4, "3-hop chain should have 4 nodes");

    // @step And the chain should include all intermediate functions in order A, B, C, D
    let slugs: Vec<&str> = function_chain
        .iter()
        .map(|n| n.get("slug").and_then(|v| v.as_str()).expect("slug"))
        .collect();
    assert_eq!(
        slugs,
        vec![
            fx.s("func_a"),
            fx.s("func_b"),
            fx.s("func_c"),
            fx.s("func_d"),
        ]
    );
}

// ============================================================================
// Scenario: No path exists between two unconnected functions
// ============================================================================
#[tokio::test]
async fn test_no_path_between_unconnected_functions() {
    // @step Given I have a codebase indexed with two functions that have no call path between them
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "isolated"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("isolated"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an empty chains array
    let chains = parsed
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(
        chains.is_empty(),
        "Should find no chains for unconnected functions"
    );

    // @step And the response should include a message indicating no path was found within the depth limit
    let message = parsed.get("message").and_then(|v| v.as_str());
    assert!(message.is_some(), "Should include a message");
    assert!(
        message.unwrap().contains("No call path found"),
        "Message should indicate no path found"
    );
}

// ============================================================================
// Scenario: Non-existent source function slug
// ============================================================================
#[tokio::test]
async fn test_nonexistent_source_function() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "nonexistent_function" to "func_b"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, "nonexistent_function", fx.s("func_b"), None)
            .await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the source function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(
        error.unwrap().contains("not found"),
        "Error should mention 'not found'"
    );
}

// ============================================================================
// Scenario: Non-existent target function slug
// ============================================================================
#[tokio::test]
async fn test_nonexistent_target_function() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "nonexistent_function"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), "nonexistent_function", None)
            .await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the target function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(
        error.unwrap().contains("not found"),
        "Error should mention 'not found'"
    );
}

// ============================================================================
// Scenario: Max depth limits path discovery
// ============================================================================
#[tokio::test]
async fn test_max_depth_limits_discovery() {
    // @step Given I have a codebase indexed where the shortest path between two functions is 3 hops
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;
    // func_a → func_b → func_c → func_d = 3 hops

    // @step When I request ast_call_chain with max_depth 2
    let result_shallow =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_d"), Some(2)).await;
    let parsed_shallow: Value = serde_json::from_str(&result_shallow).expect("valid JSON");

    // @step Then I should receive an empty chains array
    let chains_shallow = parsed_shallow
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(
        chains_shallow.is_empty(),
        "max_depth=2 should not find 3-hop path"
    );

    // @step When I request ast_call_chain with max_depth 3
    let result_deep =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_d"), Some(3)).await;
    let parsed_deep: Value = serde_json::from_str(&result_deep).expect("valid JSON");

    // @step Then I should receive a chains array containing the 3-hop path
    let chains_deep = parsed_deep
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(
        !chains_deep.is_empty(),
        "max_depth=3 should find 3-hop path"
    );
}
