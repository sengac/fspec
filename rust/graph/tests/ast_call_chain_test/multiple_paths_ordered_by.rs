// Feature: spec/features/call-chain-path-tracing-between-two-functions.feature
use super::*;

// ============================================================================
// Scenario: Multiple paths returned ordered by length
// ============================================================================
#[tokio::test]
async fn test_multiple_paths_ordered_by_length() {
    // @step Given I have a codebase indexed with both a 2-hop and a 3-hop path between two functions
    let fx = setup_multi_path_db().await;
    let db = &fx.graph;
    // entry → short_path → target (2 hops)
    // entry → long_path_a → long_path_b → target (3 hops)

    // @step When I request ast_call_chain between those two functions
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("entry"), fx.s("target"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the chains array should contain the shorter path first
    let chains = parsed
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(chains.len() >= 2, "Should find at least 2 paths");
    let first_chain_len = chains[0]
        .get("chain_length")
        .and_then(|v| v.as_u64())
        .expect("chain_length");
    let second_chain_len = chains[1]
        .get("chain_length")
        .and_then(|v| v.as_u64())
        .expect("chain_length");
    assert!(
        first_chain_len <= second_chain_len,
        "Shorter path should come first"
    );

    // @step And results should be limited to at most 20 chains
    assert!(chains.len() <= 20, "Should not exceed 20 chains");
}

// ============================================================================
// Scenario: Chain results include function metadata and call details per hop
// ============================================================================
#[tokio::test]
async fn test_chain_results_include_function_and_call_details() {
    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_b"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    let chains = parsed
        .get("chains")
        .and_then(|v| v.as_array())
        .expect("chains array");
    assert!(!chains.is_empty(), "Should find at least one chain");
    let chain = chains[0].as_object().expect("chain is object");

    // @step Then each chain should contain a function_chain array with node metadata for each function
    let function_chain = chain
        .get("function_chain")
        .and_then(|v| v.as_array())
        .expect("function_chain array");
    assert_eq!(function_chain.len(), 2, "Direct call: 2 functions");
    // Verify function metadata fields exist
    let first_fn = &function_chain[0];
    assert!(
        first_fn.get("slug").is_some(),
        "function_chain should have slug"
    );
    assert!(
        first_fn.get("name").is_some(),
        "function_chain should have name"
    );

    // @step And each chain should contain a call_details array with edge metadata for each hop
    let call_details = chain
        .get("call_details")
        .and_then(|v| v.as_array())
        .expect("call_details array");
    // 1-hop chain = 1 call detail
    assert_eq!(
        call_details.len(),
        1,
        "Direct call: 1 hop = 1 call_details entry"
    );
    // KGRAPH-078: the hop carries its Calls edge's metadata as ast_index
    // stored it (func_a calls func_b once, unconditionally).
    assert_eq!(call_details[0]["callCount"], 1, "{:?}", call_details[0]);
    assert_eq!(
        call_details[0]["isConditional"], false,
        "{:?}",
        call_details[0]
    );

    // @step And each chain should include a chain_length integer
    let chain_length = chain
        .get("chain_length")
        .and_then(|v| v.as_u64())
        .expect("chain_length integer");
    assert_eq!(chain_length, 1, "Direct call = chain_length 1");
}

// ============================================================================
// Scenario: Successful response includes human-readable summary
// ============================================================================
#[tokio::test]
async fn test_response_includes_summary() {
    // @step Given I have a codebase indexed in the AST graph with Calls edges
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_call_chain from "func_a" to "func_b"
    let result =
        ast_call_chain::dispatch_ast_call_chain(db, fx.s("func_a"), fx.s("func_b"), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the response should include a summary string describing the number of chains found
    let summary = parsed
        .get("summary")
        .and_then(|v| v.as_str())
        .expect("summary string");
    assert!(summary.contains("Found"), "Summary should contain 'Found'");
    assert!(
        summary.contains("call chain"),
        "Summary should mention 'call chain'"
    );
}
