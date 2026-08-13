// Feature: spec/features/ast-graph-query-interface-graphsearch-integration.feature
//
// AST Graph Query Interface & GraphSearch Integration
// Tests for AST-specific query actions (AstSearch, AstNeighbors, AstStats)
// routed through the GraphSearch tool infrastructure.
//
// Each test populates an isolated AST graph database with known data,
// then exercises the dispatch functions directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_dispatch;
use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create an AST graph database pre-loaded with test data.
async fn setup_test_ast_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-ast.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    // Load test data: 2 files, 3 functions, 1 type, 1 dependency
    let jsonl = r#"{"type":"File","data":{"slug":"src-auth-login-ts","path":"src/auth/login.ts","language":"typescript","lineCount":50,"isTest":false}}
{"type":"File","data":{"slug":"src-auth-utils-ts","path":"src/auth/utils.ts","language":"typescript","lineCount":30,"isTest":false}}
{"type":"Function","data":{"slug":"src-auth-login-ts::login","name":"login","qualifiedName":"src-auth-login-ts::login","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":5,"lineEnd":15}}
{"type":"Function","data":{"slug":"src-auth-login-ts::verify","name":"verify","qualifiedName":"src-auth-login-ts::verify","isAsync":false,"isPublic":false,"paramCount":2,"lineStart":17,"lineEnd":25}}
{"type":"Function","data":{"slug":"src-auth-utils-ts::hash","name":"hash","qualifiedName":"src-auth-utils-ts::hash","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":1,"lineEnd":5}}
{"type":"Type","data":{"slug":"src-auth-login-ts::UserSession","name":"UserSession","typeKind":"interface","isPublic":true}}
{"type":"Dependency","data":{"slug":"dep::express","name":"express","version":"^4.18.0","isDev":false,"source":"npm"}}
{"edge":"Contains","from":"src-auth-login-ts","to":"src-auth-login-ts::login","data":{}}
{"edge":"Contains","from":"src-auth-login-ts","to":"src-auth-login-ts::verify","data":{}}
{"edge":"Contains","from":"src-auth-utils-ts","to":"src-auth-utils-ts::hash","data":{}}
{"edge":"Calls","from":"src-auth-login-ts::login","to":"src-auth-login-ts::verify","data":{"callCount":1}}
{"edge":"Calls","from":"src-auth-login-ts::login","to":"src-auth-utils-ts::hash","data":{"callCount":1}}
{"edge":"Imports","from":"src-auth-login-ts","to":"src-auth-utils-ts","data":{"importPath":"./utils"}}
{"edge":"DependsOn","from":"src-auth-login-ts","to":"dep::express","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: Search for a function by name using AstSearch action
// ============================================================================
#[tokio::test]
async fn test_ast_search_function_by_name() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the AST graph database is initialized with File and Function nodes
    let db = setup_test_ast_db(temp_dir.path()).await;

    // @step When I execute an AstSearch action with query "login"
    let result =
        ast_dispatch::dispatch_ast_search(&db, "login", None, None, None, None, None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the result should contain a Function node matching "login"
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(!results.is_empty(), "Should find at least one result");

    let first = &results[0];

    // @step And the result should include the function's slug, name, and qualifiedName
    assert!(first.get("slug").is_some(), "Result should have slug");
    assert!(first.get("name").is_some(), "Result should have name");
    assert!(
        first.get("qualifiedName").is_some(),
        "Result should have qualifiedName"
    );

    // @step And the result should include lineStart and lineEnd positions
    assert!(
        first.get("lineStart").is_some(),
        "Result should have lineStart"
    );
    assert!(first.get("lineEnd").is_some(), "Result should have lineEnd");

    // @step And the result should include paramCount
    assert!(
        first.get("paramCount").is_some(),
        "Result should have paramCount"
    );
}

// ============================================================================
// Scenario: Get neighbors of a Function node using AstNeighbors action
// ============================================================================
#[tokio::test]
async fn test_ast_neighbors_of_function_node() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the AST graph database contains Function nodes with Contains and Calls edges
    let db = setup_test_ast_db(temp_dir.path()).await;

    // @step When I execute an AstNeighbors action for a Function node slug
    let result =
        ast_dispatch::dispatch_ast_neighbors(&db, "src-auth-login-ts::login", None, None).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");
    let neighbors = parsed
        .get("neighbors")
        .and_then(|v| v.as_array())
        .expect("neighbors array");

    // @step Then the result should include the File node that contains the function via Contains edge
    // The login function is contained by src-auth-login-ts (incoming Contains)
    let has_file = neighbors.iter().any(|n| {
        n.get("slug")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("src-auth-login-ts") && !s.contains("::"))
    });
    assert!(has_file, "Should include the containing File node");

    // @step And the result should include other Function nodes linked by Calls edges
    let has_called_fn = neighbors.iter().any(|n| {
        n.get("slug")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.contains("verify") || s.contains("hash"))
    });
    assert!(has_called_fn, "Should include called Function nodes");
}

// ============================================================================
// Scenario: Get AST codebase statistics using AstStats action
// ============================================================================
#[tokio::test]
async fn test_ast_stats_codebase_statistics() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the AST graph database contains various node and edge types
    let db = setup_test_ast_db(temp_dir.path()).await;

    // @step When I execute an AstStats action
    let result = ast_dispatch::dispatch_ast_stats(&db).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then the result should include counts for File, Function, Type, and Dependency nodes
    let nodes = parsed.get("nodes").expect("Should have nodes object");
    assert!(
        nodes.get("File").and_then(|v| v.as_u64()).unwrap_or(0) >= 2,
        "Should have at least 2 File nodes"
    );
    assert!(
        nodes.get("Function").and_then(|v| v.as_u64()).unwrap_or(0) >= 3,
        "Should have at least 3 Function nodes"
    );
    assert!(
        nodes.get("Type").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "Should have at least 1 Type node"
    );
    assert!(
        nodes
            .get("Dependency")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "Should have at least 1 Dependency node"
    );

    // @step And the result should include total edge counts
    let edges = parsed.get("edges").expect("Should have edges object");
    assert!(
        edges.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "Should have at least 1 total edge"
    );
}
