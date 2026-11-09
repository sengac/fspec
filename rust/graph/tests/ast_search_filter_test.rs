// Feature: spec/features/fulltext-content-search.feature
//
// Full-Text and Content Search Within Graph
// Tests for search_mode, decorator filter, and parameter filter on ast_search.
//
// Integration tests populate an isolated AST graph with Function/Type nodes
// that have source, docstring, parameters, and decorators, then exercise
// dispatch_ast_search with the new filter parameters.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_dispatch;
use codelet_graph::database::GraphDatabase;
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

#[path = "ast_search_filter_test/name_only_search_excludes.rs"]
mod name_only_search_excludes;
#[path = "ast_search_filter_test/query_combined_with_decorator.rs"]
mod query_combined_with_decorator;
