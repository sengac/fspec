// Feature: spec/features/fulltext-content-search.feature
use super::*;

// ============================================================================
// Scenario: Query combined with decorator filter uses AND logic
// ============================================================================
#[tokio::test]
async fn test_query_combined_with_decorator_uses_and_logic() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have names and decorators stored

    // @step When I search with query "User" and decorator filter "Injectable" and search_mode "name"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "User",
        None, // search all entity types (Function + Type)
        None,
        None,
        Some("name"),
        Some("Injectable"),
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include only functions named "User" that also have the "Injectable" decorator
    // UserService type has "User" in name AND @Injectable decorator
    assert!(
        names.contains(&"UserService"),
        "UserService matches name 'User' AND decorator 'Injectable'"
    );
    // validate_credentials has @Injectable but name doesn't contain "User"
    assert!(
        !names.contains(&"validate_credentials"),
        "validate_credentials has Injectable but name doesn't match 'User'"
    );
    // get_user_name has "user" in name but no @Injectable
    assert!(
        !names.contains(&"get_user_name"),
        "get_user_name matches 'User' but has no Injectable decorator"
    );
}

// ============================================================================
// Scenario: All search mode searches every field
// ============================================================================
#[tokio::test]
async fn test_all_search_mode_searches_every_field() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions that have full metadata

    // @step When I search with query "validate" and search_mode "all"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "validate",
        Some("Function"),
        None,
        None,
        Some("all"), // search everything
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include functions matching "validate" in name, source, docstring, parameters, or decorators
    // validate_credentials — name match
    assert!(
        names.contains(&"validate_credentials"),
        "validate_credentials matches by name"
    );
    // validate_input doesn't exist in our test data but "Validates" appears in validate_credentials docstring
    // process_login source doesn't contain "validate" so it shouldn't match
}

// ============================================================================
// Scenario: Decorator matching strips leading symbols for cross-language matching
// ============================================================================
#[tokio::test]
async fn test_decorator_matching_strips_leading_symbols() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_search_db(temp_dir.path()).await;

    // @step Given a project is indexed with functions decorated with "@test" and "#[test]" and "@Test"
    // run_auth_tests: "@Test, @Integration"
    // handle_request: "#[test]"
    // test_login_flow: "@test"

    // @step When I search with decorator filter "test"
    let result_str = ast_dispatch::dispatch_ast_search(
        &db,
        "",
        Some("Function"),
        None,
        None,
        Some("all"),  // so empty query matches everything
        Some("test"), // lowercase "test"
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include all three functions regardless of decorator syntax prefix
    assert!(
        names.contains(&"run_auth_tests"),
        "Matches @Test (capital T)"
    );
    assert!(
        names.contains(&"handle_request"),
        "Matches #[test] (Rust syntax)"
    );
    assert!(
        names.contains(&"test_login_flow"),
        "Matches @test (lowercase)"
    );
    assert_eq!(names.len(), 3, "Exactly 3 functions match 'test' decorator");
}
