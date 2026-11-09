// Feature: spec/features/fulltext-content-search.feature
use super::*;

// ============================================================================
// Scenario: Name-only search excludes source code matches
// ============================================================================
#[tokio::test]
async fn test_name_only_search_excludes_source_matches() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have source code stored
    // (graph loaded above with 7 functions + 1 type with source/docstring)

    // @step When I search with query "dispatch" and search_mode "name"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "dispatch",
        None,         // all entity types
        None,         // default limit
        None,         // no path filter
        Some("name"), // search_mode = name
        None,         // no decorator filter
        None,         // no parameter filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then results include functions whose names contain "dispatch"
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();
    assert!(
        names.contains(&"dispatch_ast_search"),
        "Should find dispatch_ast_search by name"
    );

    // @step And results do not include functions that only mention "dispatch" in their source code
    // get_user_name doesn't have "dispatch" in name — it shouldn't appear
    assert!(
        !names.contains(&"get_user_name"),
        "Should not find get_user_name (no 'dispatch' in name)"
    );
}

// ============================================================================
// Scenario: Content search finds matches in source code and docstrings
// ============================================================================
#[tokio::test]
async fn test_content_search_finds_source_and_docstring_matches() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have source code and docstrings stored
    // (graph loaded above)

    // @step When I search with query "authentication" and search_mode "content"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "authentication",
        Some("Function"),
        None,
        None,
        Some("content"), // search_mode = content (source + docstring only)
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include functions whose source code contains "authentication"
    // dispatch_ast_search source: "performs authentication check"
    // process_login source: "authentication flow"
    assert!(
        names.contains(&"dispatch_ast_search"),
        "dispatch_ast_search source mentions authentication"
    );
    assert!(
        names.contains(&"process_login"),
        "process_login source mentions authentication"
    );

    // @step And results include functions whose docstrings contain "authentication"
    // validate_credentials docstring: "Validates user authentication credentials"
    assert!(
        names.contains(&"validate_credentials"),
        "validate_credentials docstring mentions authentication"
    );

    // @step And results do not include functions that only match "authentication" in their name
    // get_user_name has no "authentication" in source or docstring
    assert!(
        !names.contains(&"get_user_name"),
        "get_user_name has no authentication in content"
    );
}

// ============================================================================
// Scenario: Default search mode is name-only for backward compatibility
// ============================================================================
#[tokio::test]
async fn test_default_search_mode_is_name_only() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have metadata stored
    // (graph loaded above)

    // @step When I search with query "process" and no search_mode parameter
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "process",
        Some("Function"),
        None,
        None,
        None, // no search_mode — defaults to "name"
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results only include entities whose name, slug, path, or qualifiedName contains "process"
    // process_login has "process" in its name
    assert!(
        names.contains(&"process_login"),
        "process_login matches by name"
    );
    // dispatch_ast_search does NOT have "process" in name/slug/path/qualifiedName
    // (its source mentions "process" but name mode shouldn't match that)
    // handle_request source has "process(request)" but name doesn't contain "process"
    assert!(
        !names.contains(&"handle_request"),
        "handle_request source mentions process but name mode skips source"
    );
}

// ============================================================================
// Scenario: Decorator filter returns functions with matching decorator
// ============================================================================
#[tokio::test]
async fn test_decorator_filter_returns_matching_functions() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have decorators stored
    // (graph loaded above — some functions have @Test, @Injectable, @Post, #[test], @test)

    // @step When I search with decorator filter "Test"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "", // empty query matches everything in name mode when combined with decorator filter
        None,
        None,
        None,
        Some("all"),  // use "all" so empty query doesn't filter out
        Some("Test"), // decorator filter
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include only functions whose decorators contain "Test"
    assert!(
        names.contains(&"run_auth_tests"),
        "run_auth_tests has @Test decorator"
    );
    assert!(
        names.contains(&"handle_request"),
        "handle_request has #[test] decorator"
    );
    assert!(
        names.contains(&"test_login_flow"),
        "test_login_flow has @test decorator"
    );

    // @step And decorator matching is case-insensitive
    // "Test" matches "@Test", "#[test]", "@test" — all three found above
    assert!(
        !names.contains(&"validate_credentials"),
        "validate_credentials has @Injectable, not Test"
    );
    assert!(
        !names.contains(&"get_user_name"),
        "get_user_name has no decorators"
    );
}

// ============================================================================
// Scenario: Parameter filter returns functions with matching parameter name
// ============================================================================
#[tokio::test]
async fn test_parameter_filter_returns_matching_functions() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have parameters stored
    // (graph loaded above — validate_credentials and handle_request both have "request" param)

    // @step When I search with parameter filter "request"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "",
        Some("Function"),
        None,
        None,
        Some("all"),
        None,
        Some("request"), // parameter filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include only functions whose parameters contain "request"
    assert!(
        names.contains(&"validate_credentials"),
        "validate_credentials has 'request' param"
    );
    assert!(
        names.contains(&"handle_request"),
        "handle_request has 'request' param"
    );
    // Others should NOT be in results
    assert!(
        !names.contains(&"get_user_name"),
        "get_user_name has 'user_id' param, not 'request'"
    );
    assert!(
        !names.contains(&"run_auth_tests"),
        "run_auth_tests has 'ctx' param, not 'request'"
    );
}
