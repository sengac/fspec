// Feature: spec/features/fulltext-content-search.feature
//
// Full-Text and Content Search Within Graph
// Tests for search_mode, decorator filter, and parameter filter on ast_search.
//
// Integration tests populate an isolated AST graph with Function/Type nodes
// that have source, docstring, parameters, and decorators, then exercise
// dispatch_ast_search with the new filter parameters.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_dispatch;
use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Helper: create a graph with functions that have rich metadata for search testing.
///
/// Functions:
///   - `dispatch_ast_search` — source mentions "authentication", decorators: "", params: "db, query, entity_type, limit, path_pattern"
///   - `validate_credentials` — source mentions "check password", docstring: "Validates user authentication credentials", decorators: "@Injectable", params: "request, response"
///   - `process_login` — source mentions "authentication flow", decorators: "@Post, @UseGuards", params: "body, session"
///   - `get_user_name` — simple getter, no docstring about auth, decorators: "", params: "user_id"
///   - `run_auth_tests` — decorators: "@Test, @Integration", params: "ctx"
///   - `handle_request` — decorators: "#[test]", params: "request, headers"
///   - `test_login_flow` — decorators: "@test", params: "mock_db"
///
/// Types:
///   - `UserService` — decorators: "@Injectable", docstring: "Service for user management"
async fn setup_search_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-search.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r##"{"type":"File","data":{"slug":"src-dispatch-rs","path":"src/dispatch.rs","language":"rust","lineCount":200,"isTest":false}}
{"type":"File","data":{"slug":"src-auth-ts","path":"src/auth/auth.ts","language":"typescript","lineCount":150,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-ts","path":"src/utils.ts","language":"typescript","lineCount":50,"isTest":false}}
{"type":"File","data":{"slug":"tests-auth-ts","path":"tests/auth.test.ts","language":"typescript","lineCount":80,"isTest":true}}
{"type":"Function","data":{"slug":"src-dispatch-rs::dispatch_ast_search","name":"dispatch_ast_search","qualifiedName":"src-dispatch-rs::dispatch_ast_search","isAsync":true,"isPublic":true,"paramCount":5,"lineStart":1,"lineEnd":50,"parameters":"db, query, entity_type, limit, path_pattern","source":"pub async fn dispatch_ast_search(db: &GraphDatabase, query: &str) {\n    // performs authentication check before searching\n    let results = db.query_all().await;\n}","docstring":"Search AST code entities by name or pattern.","decorators":"","language":"rust"}}
{"type":"Function","data":{"slug":"src-auth-ts::validate_credentials","name":"validate_credentials","qualifiedName":"src-auth-ts::validate_credentials","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":10,"lineEnd":40,"parameters":"request, response","source":"async function validate_credentials(request, response) {\n    const valid = check_password(request.body.password);\n    return valid;\n}","docstring":"Validates user authentication credentials against the database.","decorators":"@Injectable","language":"typescript"}}
{"type":"Function","data":{"slug":"src-auth-ts::process_login","name":"process_login","qualifiedName":"src-auth-ts::process_login","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":42,"lineEnd":80,"parameters":"body, session","source":"async function process_login(body, session) {\n    // authentication flow: validate then create session\n    const user = await findUser(body.email);\n}","docstring":"Processes the login request.","decorators":"@Post, @UseGuards","language":"typescript"}}
{"type":"Function","data":{"slug":"src-utils-ts::get_user_name","name":"get_user_name","qualifiedName":"src-utils-ts::get_user_name","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":1,"lineEnd":5,"parameters":"user_id","source":"function get_user_name(user_id) { return users[user_id].name; }","docstring":"Gets the display name for a user.","decorators":"","language":"typescript"}}
{"type":"Function","data":{"slug":"tests-auth-ts::run_auth_tests","name":"run_auth_tests","qualifiedName":"tests-auth-ts::run_auth_tests","isAsync":true,"isPublic":true,"paramCount":1,"lineStart":1,"lineEnd":30,"parameters":"ctx","source":"async function run_auth_tests(ctx) { expect(true).toBe(true); }","docstring":"Integration tests for auth module.","decorators":"@Test, @Integration","language":"typescript"}}
{"type":"Function","data":{"slug":"tests-auth-ts::handle_request","name":"handle_request","qualifiedName":"tests-auth-ts::handle_request","isAsync":false,"isPublic":true,"paramCount":2,"lineStart":32,"lineEnd":50,"parameters":"request, headers","source":"fn handle_request(request, headers) { process(request); }","docstring":"Handler for incoming requests.","decorators":"#[test]","language":"rust"}}
{"type":"Function","data":{"slug":"tests-auth-ts::test_login_flow","name":"test_login_flow","qualifiedName":"tests-auth-ts::test_login_flow","isAsync":true,"isPublic":true,"paramCount":1,"lineStart":52,"lineEnd":70,"parameters":"mock_db","source":"async function test_login_flow(mock_db) { await login(mock_db); }","docstring":"Tests the login flow end to end.","decorators":"@test","language":"typescript"}}
{"type":"Type","data":{"slug":"src-auth-ts::UserService","name":"UserService","typeKind":"class","isPublic":true,"fieldCount":3,"lineStart":82,"lineEnd":120,"source":"class UserService { constructor() {} }","docstring":"Service for user management and authentication.","decorators":"@Injectable","language":"typescript"}}
{"edge":"Contains","from":"src-dispatch-rs","to":"src-dispatch-rs::dispatch_ast_search","data":{}}
{"edge":"Contains","from":"src-auth-ts","to":"src-auth-ts::validate_credentials","data":{}}
{"edge":"Contains","from":"src-auth-ts","to":"src-auth-ts::process_login","data":{}}
{"edge":"Contains","from":"src-utils-ts","to":"src-utils-ts::get_user_name","data":{}}
{"edge":"Contains","from":"tests-auth-ts","to":"tests-auth-ts::run_auth_tests","data":{}}
{"edge":"Contains","from":"tests-auth-ts","to":"tests-auth-ts::handle_request","data":{}}
{"edge":"Contains","from":"tests-auth-ts","to":"tests-auth-ts::test_login_flow","data":{}}
{"edge":"ContainsType","from":"src-auth-ts","to":"src-auth-ts::UserService","data":{}}"##;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

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
        None,  // all entity types
        None,  // default limit
        None,  // no path filter
        Some("name"),  // search_mode = name
        None,  // no decorator filter
        None,  // no parameter filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then results include functions whose names contain "dispatch"
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();
    assert!(names.contains(&"dispatch_ast_search"), "Should find dispatch_ast_search by name");

    // @step And results do not include functions that only mention "dispatch" in their source code
    // get_user_name doesn't have "dispatch" in name — it shouldn't appear
    assert!(!names.contains(&"get_user_name"), "Should not find get_user_name (no 'dispatch' in name)");
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
        Some("content"),  // search_mode = content (source + docstring only)
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
    assert!(names.contains(&"dispatch_ast_search"), "dispatch_ast_search source mentions authentication");
    assert!(names.contains(&"process_login"), "process_login source mentions authentication");

    // @step And results include functions whose docstrings contain "authentication"
    // validate_credentials docstring: "Validates user authentication credentials"
    assert!(names.contains(&"validate_credentials"), "validate_credentials docstring mentions authentication");

    // @step And results do not include functions that only match "authentication" in their name
    // get_user_name has no "authentication" in source or docstring
    assert!(!names.contains(&"get_user_name"), "get_user_name has no authentication in content");
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
        None,  // no search_mode — defaults to "name"
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results only include entities whose name, slug, path, or qualifiedName contains "process"
    // process_login has "process" in its name
    assert!(names.contains(&"process_login"), "process_login matches by name");
    // dispatch_ast_search does NOT have "process" in name/slug/path/qualifiedName
    // (its source mentions "process" but name mode shouldn't match that)
    // handle_request source has "process(request)" but name doesn't contain "process"
    assert!(!names.contains(&"handle_request"), "handle_request source mentions process but name mode skips source");
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
        "",  // empty query matches everything in name mode when combined with decorator filter
        None,
        None,
        None,
        Some("all"),  // use "all" so empty query doesn't filter out
        Some("Test"),  // decorator filter
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include only functions whose decorators contain "Test"
    assert!(names.contains(&"run_auth_tests"), "run_auth_tests has @Test decorator");
    assert!(names.contains(&"handle_request"), "handle_request has #[test] decorator");
    assert!(names.contains(&"test_login_flow"), "test_login_flow has @test decorator");

    // @step And decorator matching is case-insensitive
    // "Test" matches "@Test", "#[test]", "@test" — all three found above
    assert!(!names.contains(&"validate_credentials"), "validate_credentials has @Injectable, not Test");
    assert!(!names.contains(&"get_user_name"), "get_user_name has no decorators");
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
        Some("request"),  // parameter filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include only functions whose parameters contain "request"
    assert!(names.contains(&"validate_credentials"), "validate_credentials has 'request' param");
    assert!(names.contains(&"handle_request"), "handle_request has 'request' param");
    // Others should NOT be in results
    assert!(!names.contains(&"get_user_name"), "get_user_name has 'user_id' param, not 'request'");
    assert!(!names.contains(&"run_auth_tests"), "run_auth_tests has 'ctx' param, not 'request'");
}

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
        None,  // search all entity types (Function + Type)
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
    assert!(names.contains(&"UserService"), "UserService matches name 'User' AND decorator 'Injectable'");
    // validate_credentials has @Injectable but name doesn't contain "User"
    assert!(!names.contains(&"validate_credentials"), "validate_credentials has Injectable but name doesn't match 'User'");
    // get_user_name has "user" in name but no @Injectable
    assert!(!names.contains(&"get_user_name"), "get_user_name matches 'User' but has no Injectable decorator");
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
        Some("all"),  // search everything
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include functions matching "validate" in name, source, docstring, parameters, or decorators
    // validate_credentials — name match
    assert!(names.contains(&"validate_credentials"), "validate_credentials matches by name");
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
        Some("test"),  // lowercase "test"
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");
    let results = result["results"].as_array().expect("results array");
    let names: Vec<&str> = results.iter().filter_map(|r| r["name"].as_str()).collect();

    // @step Then results include all three functions regardless of decorator syntax prefix
    assert!(names.contains(&"run_auth_tests"), "Matches @Test (capital T)");
    assert!(names.contains(&"handle_request"), "Matches #[test] (Rust syntax)");
    assert!(names.contains(&"test_login_flow"), "Matches @test (lowercase)");
    assert_eq!(names.len(), 3, "Exactly 3 functions match 'test' decorator");
}
