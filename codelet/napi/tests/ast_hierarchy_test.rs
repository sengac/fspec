// Feature: spec/features/class-hierarchy-and-inheritance-traversal.feature
//
// Class Hierarchy and Inheritance Traversal
// Tests for ast_hierarchy action that returns the full inheritance tree
// (parents, children, interfaces, methods) for a type.
//
// Each test populates an isolated AST graph database with known hierarchy data,
// then exercises the dispatch function directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_hierarchy;
use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create a graph with a class hierarchy:
///
/// ```
/// Animal (parent)
///   └── Dog (child, implements Trainable)
///         └── GuideDog (grandchild)
/// ```
///
/// Plus methods for each class and the Trainable interface.
async fn setup_hierarchy_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-hierarchy.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-animals-py","path":"src/animals.py","language":"python","lineCount":100,"isTest":false}}
{"type":"Type","data":{"slug":"src-animals-py::Animal","name":"Animal","typeKind":"class","isPublic":true}}
{"type":"Type","data":{"slug":"src-animals-py::Dog","name":"Dog","typeKind":"class","isPublic":true}}
{"type":"Type","data":{"slug":"src-animals-py::GuideDog","name":"GuideDog","typeKind":"class","isPublic":true}}
{"type":"Type","data":{"slug":"src-animals-py::Trainable","name":"Trainable","typeKind":"interface","isPublic":true}}
{"type":"Function","data":{"slug":"src-animals-py::speak","name":"speak","qualifiedName":"src-animals-py::speak","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":5,"lineEnd":7}}
{"type":"Function","data":{"slug":"src-animals-py::fetch","name":"fetch","qualifiedName":"src-animals-py::fetch","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":15,"lineEnd":18}}
{"type":"Function","data":{"slug":"src-animals-py::guide","name":"guide","qualifiedName":"src-animals-py::guide","isAsync":false,"isPublic":true,"paramCount":2,"lineStart":25,"lineEnd":30}}
{"edge":"ContainsType","from":"src-animals-py","to":"src-animals-py::Animal","data":{}}
{"edge":"ContainsType","from":"src-animals-py","to":"src-animals-py::Dog","data":{}}
{"edge":"ContainsType","from":"src-animals-py","to":"src-animals-py::GuideDog","data":{}}
{"edge":"ContainsType","from":"src-animals-py","to":"src-animals-py::Trainable","data":{}}
{"edge":"Contains","from":"src-animals-py","to":"src-animals-py::speak","data":{}}
{"edge":"Contains","from":"src-animals-py","to":"src-animals-py::fetch","data":{}}
{"edge":"Contains","from":"src-animals-py","to":"src-animals-py::guide","data":{}}
{"edge":"Extends","from":"src-animals-py::Dog","to":"src-animals-py::Animal","data":{}}
{"edge":"Extends","from":"src-animals-py::GuideDog","to":"src-animals-py::Dog","data":{}}
{"edge":"Implements","from":"src-animals-py::Dog","to":"src-animals-py::Trainable","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

/// Helper: create a graph with a standalone class (no parents, no children).
async fn setup_standalone_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-standalone.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-util-py","path":"src/util.py","language":"python","lineCount":50,"isTest":false}}
{"type":"Type","data":{"slug":"src-util-py::Formatter","name":"Formatter","typeKind":"class","isPublic":true}}
{"type":"Function","data":{"slug":"src-util-py::format_text","name":"format_text","qualifiedName":"src-util-py::format_text","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":3,"lineEnd":8}}
{"edge":"ContainsType","from":"src-util-py","to":"src-util-py::Formatter","data":{}}
{"edge":"Contains","from":"src-util-py","to":"src-util-py::format_text","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: Find class hierarchy with parents and children
// ============================================================================
#[tokio::test]
async fn test_hierarchy_parents_children_interfaces() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_hierarchy_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed with class inheritance relationships
    // (DB populated with Animal → Dog → GuideDog hierarchy, Dog implements Trainable)

    // @step When I request ast_hierarchy for a class with parent and child classes
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &db,
        "src-animals-py::Dog",
        None, // default depth
        None, // include_methods default true
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");

    // Verify no error
    assert!(result.get("error").is_none(), "Expected no error, got: {result_json}");

    // @step Then I should receive the parent classes via Extends edges
    let parents = result["parents"].as_array().expect("parents array");
    assert!(
        parents.iter().any(|p| p["name"] == "Animal"),
        "Should find Animal as parent, got: {parents:?}"
    );

    // @step And I should receive the child classes via reverse Extends edges
    let children = result["children"].as_array().expect("children array");
    assert!(
        children.iter().any(|c| c["name"] == "GuideDog"),
        "Should find GuideDog as child, got: {children:?}"
    );

    // @step And I should receive implemented interfaces via Implements edges
    let interfaces = result["interfaces"].as_array().expect("interfaces array");
    assert!(
        interfaces.iter().any(|i| i["name"] == "Trainable"),
        "Should find Trainable as interface, got: {interfaces:?}"
    );

    // @step And each class should include its methods
    let methods = result["methods"].as_array().expect("methods array");
    assert!(
        !methods.is_empty(),
        "Should include methods for the type's file"
    );
}

// ============================================================================
// Scenario: Multi-level hierarchy traversal
// ============================================================================
#[tokio::test]
async fn test_multi_level_hierarchy() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_hierarchy_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed with a 3-level class hierarchy
    // (Animal → Dog → GuideDog)

    // @step When I request ast_hierarchy for the middle class
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &db,
        "src-animals-py::Dog",
        Some(3), // depth 3 to ensure multi-level
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");
    assert!(result.get("error").is_none(), "Expected no error, got: {result_json}");

    // @step Then I should receive grandparent classes in the parents array
    // Dog → Animal is direct parent at depth 1
    let parents = result["parents"].as_array().expect("parents array");
    assert!(
        parents.iter().any(|p| p["name"] == "Animal"),
        "Should find Animal (grandparent accessible via Dog)",
    );

    // @step And I should receive grandchild classes in the children array
    let children = result["children"].as_array().expect("children array");
    assert!(
        children.iter().any(|c| c["name"] == "GuideDog"),
        "Should find GuideDog (child of Dog)",
    );
}

// ============================================================================
// Scenario: Standalone type with no inheritance
// ============================================================================
#[tokio::test]
async fn test_standalone_type_no_inheritance() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_standalone_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed with a class that has no parents or children
    // (Formatter class with no extends/implements edges)

    // @step When I request ast_hierarchy for that class
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &db,
        "src-util-py::Formatter",
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");
    assert!(result.get("error").is_none(), "Expected no error, got: {result_json}");

    // @step Then I should receive the type itself with its methods
    assert_eq!(result["type"]["name"], "Formatter");
    let methods = result["methods"].as_array().expect("methods array");
    assert!(
        methods.iter().any(|m| m["name"] == "format_text"),
        "Should include format_text method"
    );

    // @step And the parents array should be empty
    let parents = result["parents"].as_array().expect("parents array");
    assert!(parents.is_empty(), "Parents should be empty for standalone type");

    // @step And the children array should be empty
    let children = result["children"].as_array().expect("children array");
    assert!(children.is_empty(), "Children should be empty for standalone type");
}

// ============================================================================
// Scenario: Non-existent type returns error
// ============================================================================
#[tokio::test]
async fn test_nonexistent_type_returns_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_standalone_db(temp_dir.path()).await;

    // @step Given I have a codebase indexed in the AST graph
    // (any indexed graph — we use standalone_db)

    // @step When I request ast_hierarchy for a non-existent type slug
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &db,
        "nonexistent::FakeType",
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");

    // @step Then I should receive an error indicating the type was not found
    let error = result["error"].as_str().expect("error field");
    assert!(
        error.contains("not found") || error.contains("Not found"),
        "Error should indicate type not found, got: {error}"
    );
}
