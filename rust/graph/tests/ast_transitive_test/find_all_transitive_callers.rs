// Feature: spec/features/transitive-callers-callees-multi-hop-traversal.feature
use super::*;

// ============================================================================
// Scenario: Find all transitive callers of a function
// ============================================================================
#[tokio::test]
async fn test_find_all_transitive_callers() {
    // @step Given I have a codebase indexed with multi-level call chains
    // Chain: func_a → func_b → func_c → func_d
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callers for a deeply-called function
    // func_d is called by func_c (depth 1), which is called by func_b (depth 2),
    // which is called by func_a (depth 3)
    let result = ast_transitive::dispatch_ast_callers(db, fx.s("func_d"), None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a list of all direct and transitive callers
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert_eq!(
        results.len(),
        3,
        "func_d has 3 callers: func_c (d1), func_b (d2), func_a (d3)"
    );

    // @step And each caller should include its depth from the target function
    for r in results {
        let depth = r.get("depth").and_then(|v| v.as_u64());
        assert!(depth.is_some(), "Each result should have a depth field");
    }

    // @step And depth 1 callers should be the direct callers
    let depth_1: Vec<&Value> = results
        .iter()
        .filter(|r| r.get("depth").and_then(|v| v.as_u64()) == Some(1))
        .collect();
    assert_eq!(depth_1.len(), 1, "Should have exactly 1 direct caller");
    assert_eq!(
        depth_1[0].get("slug").and_then(|v| v.as_str()),
        Some(fx.s("func_c")),
        "Direct caller of func_d is func_c"
    );

    // @step And depth 2+ callers should be the transitive callers
    let depth_2_plus: Vec<&Value> = results
        .iter()
        .filter(|r| r.get("depth").and_then(|v| v.as_u64()).unwrap_or(0) >= 2)
        .collect();
    assert_eq!(
        depth_2_plus.len(),
        2,
        "Should have 2 transitive callers (depth 2+)"
    );
}

// ============================================================================
// Scenario: Find all transitive callees of an entry point
// ============================================================================
#[tokio::test]
async fn test_find_all_transitive_callees() {
    // @step Given I have a codebase indexed with multi-level call chains
    // Chain: func_a → func_b → func_c → func_d
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callees for a high-level entry point function
    let result = ast_transitive::dispatch_ast_callees(db, fx.s("func_a"), None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive a list of all functions it transitively calls
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert_eq!(
        results.len(),
        3,
        "func_a calls: func_b (d1), func_c (d2), func_d (d3)"
    );

    // @step And each callee should include slug, name, file path, line numbers, and depth
    let first = &results[0];
    assert!(first.get("slug").is_some(), "Should have slug");
    assert!(first.get("name").is_some(), "Should have name");
    assert!(first.get("path").is_some(), "Should have file path");
    assert!(first.get("depth").is_some(), "Should have depth");
    // lineStart/lineEnd come from GraphSnapshot metadata
    assert!(first.get("lineStart").is_some(), "Should have lineStart");
    assert!(first.get("lineEnd").is_some(), "Should have lineEnd");
}

// ============================================================================
// Scenario: Function with no callers returns empty results
// ============================================================================
#[tokio::test]
async fn test_function_with_no_callers_returns_empty() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callers for a function that is never called
    // func_a is at the top of the chain — no one calls it
    let result = ast_transitive::dispatch_ast_callers(db, fx.s("func_a"), None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an empty results array
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(
        results.is_empty(),
        "func_a has no callers, results should be empty"
    );
}

// ============================================================================
// Scenario: Function with no callees returns empty results
// ============================================================================
#[tokio::test]
async fn test_function_with_no_callees_returns_empty() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callees for a leaf function that calls nothing
    // func_d is at the bottom of the chain — it calls nothing
    let result = ast_transitive::dispatch_ast_callees(db, fx.s("func_d"), None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an empty results array
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(
        results.is_empty(),
        "func_d calls nothing, results should be empty"
    );
}

// ============================================================================
// Scenario: Max depth limits transitive traversal
// ============================================================================
#[tokio::test]
async fn test_max_depth_limits_transitive_traversal() {
    // @step Given I have a codebase indexed with a call chain of depth 4
    // entry → step1 → step2 → step3 → leaf (4 hops)
    let fx = setup_deep_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callees with max_depth 2
    let result = ast_transitive::dispatch_ast_callees(db, fx.s("entry"), Some(2), None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should only receive callees within 2 hops
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert_eq!(
        results.len(),
        2,
        "max_depth=2 should find step1 (d1) and step2 (d2)"
    );

    // @step And functions at depth 3 and beyond should not appear in results
    let slugs: Vec<&str> = results
        .iter()
        .filter_map(|r| r.get("slug").and_then(|v| v.as_str()))
        .collect();
    assert!(
        !slugs.contains(&fx.s("step3")),
        "step3 (depth 3) should not appear with max_depth=2"
    );
    assert!(
        !slugs.contains(&fx.s("leaf")),
        "leaf (depth 4) should not appear with max_depth=2"
    );
    // Verify correct functions are present
    assert!(
        slugs.contains(&fx.s("step1")),
        "step1 (depth 1) should be present"
    );
    assert!(
        slugs.contains(&fx.s("step2")),
        "step2 (depth 2) should be present"
    );
}

// ============================================================================
// Scenario: Non-existent function returns error
// ============================================================================
#[tokio::test]
async fn test_nonexistent_function_returns_error_callers() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callers for a non-existent function slug
    let result = ast_transitive::dispatch_ast_callers(db, "nonexistent_function", None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(
        error.unwrap().contains("not found"),
        "Error should mention 'not found'"
    );
}

#[tokio::test]
async fn test_nonexistent_function_returns_error_callees() {
    // @step Given I have a codebase indexed in the AST graph
    let fx = setup_linear_chain_db().await;
    let db = &fx.graph;

    // @step When I request ast_callees for a non-existent function slug
    let result = ast_transitive::dispatch_ast_callees(db, "nonexistent_function", None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive an error indicating the function was not found
    let error = parsed.get("error").and_then(|v| v.as_str());
    assert!(error.is_some(), "Should return error");
    assert!(
        error.unwrap().contains("not found"),
        "Error should mention 'not found'"
    );
}
