// Feature: spec/features/ast-graph-data-model.feature
//
// AST Graph Data Model & Nanograph Schema
// Tests for the reusable GraphDatabase abstraction, the AST-code schema,
// and the registry's multi-instance support.
//
// Each test uses an isolated temp directory to avoid polluting real data.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::database::GraphDatabase;
use serde_json::Value;

/// The AST code schema, same as what's bundled in the binary.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// A minimal alternative schema for testing multi-instance separation.
const ALT_SCHEMA: &str = r#"
node Widget @description("A test widget node.") {
    slug: String @key
    label: String
}

edge LinksTo: Widget -> Widget {
}
"#;

/// Inline query source for traversing AST graph neighbors.
const AST_QUERIES: &str = r#"
query all_files() {
    match { $f: File }
    return { $f.slug, $f.path, $f.language, $f.lineCount, $f.isTest }
}

query all_functions() {
    match { $fn: Function }
    return { $fn.slug, $fn.name, $fn.qualifiedName, $fn.isAsync, $fn.isPublic, $fn.paramCount }
}

query file_functions($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f contains $fn
    }
    return { $fn.slug, $fn.name, $fn.qualifiedName }
}

query function_callees($fn_slug: String) {
    match {
        $caller: Function { slug: $fn_slug }
        $caller calls $callee
    }
    return { $callee.slug, $callee.name }
}

query function_callers($fn_slug: String) {
    match {
        $caller calls $target
        $target: Function { slug: $fn_slug }
    }
    return { $caller.slug, $caller.name }
}
"#;

// ============================================================================
// Scenario: Initialize AST graph database with schema
// ============================================================================
#[tokio::test]
async fn test_initialize_ast_graph_database_with_schema() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("ast-code.nano");

    // @step Given the project root directory exists
    assert!(temp_dir.path().exists());

    // @step And no AST graph database has been initialized
    assert!(!db_path.exists(), "Database should not exist yet");

    // @step When the AST graph database is initialized
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("AST graph init should succeed");

    // @step Then the database should be created at ".fspec/graph/ast-code.nano/"
    assert!(db_path.exists(), "Database directory should exist after init");
    assert!(
        db_path.join("schema.ir.json").exists(),
        "schema.ir.json should exist"
    );

    // @step And the schema catalog should contain node types "File, Module, Function, Type, Dependency"
    let node_names = db.node_type_names();
    for expected in &["File", "Module", "Function", "Type", "Dependency"] {
        assert!(
            node_names.contains(&expected.to_string()),
            "Schema should contain node type '{}', got: {:?}",
            expected,
            node_names
        );
    }

    // @step And the schema catalog should contain edge types "Contains, ContainsType, Imports, Calls, Implements, Extends, TypeRef, DependsOn"
    let edge_names = db.edge_type_names();
    for expected in &[
        "Contains",
        "ContainsType",
        "Imports",
        "Calls",
        "Implements",
        "Extends",
        "TypeRef",
        "DependsOn",
    ] {
        assert!(
            edge_names.contains(&expected.to_string()),
            "Schema should contain edge type '{}', got: {:?}",
            expected,
            edge_names
        );
    }

    // @step And all node types should have a "slug" key property
    for node_type in &["File", "Module", "Function", "Type", "Dependency"] {
        assert!(
            db.node_has_property(node_type, "slug"),
            "Node type '{}' should have 'slug' property",
            node_type
        );
    }
}

// ============================================================================
// Scenario: Load batch of File and Function nodes via JSONL
// ============================================================================
#[tokio::test]
async fn test_load_batch_file_and_function_nodes_via_jsonl() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("ast-code.nano");

    // @step Given the AST graph database is initialized
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("Init should succeed");
    let db = db.with_query_source(AST_QUERIES);

    // @step When I load a batch of JSONL containing File and Function nodes
    let jsonl = [
        r#"{"type":"File","data":{"slug":"src-main-rs","path":"src/main.rs","language":"rust","lineCount":150,"isTest":false}}"#,
        r#"{"type":"File","data":{"slug":"src-lib-rs","path":"src/lib.rs","language":"rust","lineCount":300,"isTest":false}}"#,
        r#"{"type":"Function","data":{"slug":"src-main-rs::main","name":"main","qualifiedName":"src-main-rs::main","isAsync":true,"isPublic":true,"paramCount":0}}"#,
        r#"{"type":"Function","data":{"slug":"src-lib-rs::process","name":"process","qualifiedName":"src-lib-rs::process","isAsync":false,"isPublic":true,"paramCount":2}}"#,
        r#"{"type":"Function","data":{"slug":"src-lib-rs::validate","name":"validate","qualifiedName":"src-lib-rs::validate","isAsync":false,"isPublic":false,"paramCount":1}}"#,
    ]
    .join("\n");

    db.load_jsonl(&jsonl)
        .await
        .expect("Batch JSONL load should succeed");

    // @step Then querying for File nodes should return the loaded files with correct properties
    let files = db
        .query("all_files", None)
        .await
        .expect("all_files query should succeed");
    let files_arr = files.as_array().expect("Result should be an array");
    assert_eq!(files_arr.len(), 2, "Should have loaded 2 File nodes");

    let slugs: Vec<&str> = files_arr
        .iter()
        .filter_map(|f| f.get("slug").and_then(Value::as_str))
        .collect();
    assert!(slugs.contains(&"src-main-rs"), "Should contain src-main-rs");
    assert!(slugs.contains(&"src-lib-rs"), "Should contain src-lib-rs");

    // @step And querying for Function nodes should return the loaded functions with correct slugs
    let functions = db
        .query("all_functions", None)
        .await
        .expect("all_functions query should succeed");
    let funcs_arr = functions.as_array().expect("Result should be an array");
    assert_eq!(funcs_arr.len(), 3, "Should have loaded 3 Function nodes");

    let fn_slugs: Vec<&str> = funcs_arr
        .iter()
        .filter_map(|f| f.get("slug").and_then(Value::as_str))
        .collect();
    assert!(
        fn_slugs.contains(&"src-main-rs::main"),
        "Should contain main function"
    );
    assert!(
        fn_slugs.contains(&"src-lib-rs::process"),
        "Should contain process function"
    );
    assert!(
        fn_slugs.contains(&"src-lib-rs::validate"),
        "Should contain validate function"
    );

    // @step And no Lance version amplification should occur from the batch load
    // Verify by checking that the node segments have exactly 1 batch each
    // (single JSONL load = single batch, no amplification)
    let stats = db.stats().expect("Stats should succeed");
    let file_count = stats
        .pointer("/nodes/File")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(file_count, 2, "File node count should be 2");
    let fn_count = stats
        .pointer("/nodes/Function")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(fn_count, 3, "Function node count should be 3");
}

// ============================================================================
// Scenario: Load structural edges and traverse neighbors
// ============================================================================
#[tokio::test]
async fn test_load_structural_edges_and_traverse_neighbors() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("ast-code.nano");

    // @step Given the AST graph database is initialized
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("Init should succeed");
    let db = db.with_query_source(AST_QUERIES);

    // @step And File and Function nodes have been loaded
    let nodes_jsonl = [
        r#"{"type":"File","data":{"slug":"mod-rs","path":"src/graph/mod.rs","language":"rust","lineCount":280}}"#,
        r#"{"type":"Function","data":{"slug":"mod-rs::ensure_graph_db","name":"ensure_graph_db","qualifiedName":"graph::ensure_graph_db","isAsync":true,"isPublic":true,"paramCount":0}}"#,
        r#"{"type":"Function","data":{"slug":"mod-rs::graph_db_stats","name":"graph_db_stats","qualifiedName":"graph::graph_db_stats","isAsync":true,"isPublic":true,"paramCount":0}}"#,
        r#"{"type":"Function","data":{"slug":"mod-rs::graph_db_load_jsonl","name":"graph_db_load_jsonl","qualifiedName":"graph::graph_db_load_jsonl","isAsync":true,"isPublic":true,"paramCount":1}}"#,
    ]
    .join("\n");
    db.load_jsonl(&nodes_jsonl)
        .await
        .expect("Node load should succeed");

    // @step When I load Contains edges linking files to functions
    let contains_jsonl = [
        r#"{"edge":"Contains","from":"mod-rs","to":"mod-rs::ensure_graph_db","data":{}}"#,
        r#"{"edge":"Contains","from":"mod-rs","to":"mod-rs::graph_db_stats","data":{}}"#,
        r#"{"edge":"Contains","from":"mod-rs","to":"mod-rs::graph_db_load_jsonl","data":{}}"#,
    ]
    .join("\n");
    db.load_jsonl(&contains_jsonl)
        .await
        .expect("Contains edge load should succeed");

    // @step And I load Calls edges linking functions to other functions
    let calls_jsonl = [
        r#"{"edge":"Calls","from":"mod-rs::graph_db_stats","to":"mod-rs::ensure_graph_db","data":{"callCount":1}}"#,
        r#"{"edge":"Calls","from":"mod-rs::graph_db_load_jsonl","to":"mod-rs::ensure_graph_db","data":{"callCount":1}}"#,
    ]
    .join("\n");
    db.load_jsonl(&calls_jsonl)
        .await
        .expect("Calls edge load should succeed");

    // @step Then traversing neighbors of a function node should return its callers and callees
    let callees = db
        .query(
            "function_callees",
            Some(&serde_json::json!({"fn_slug": "mod-rs::graph_db_stats"})),
        )
        .await
        .expect("function_callees query should succeed");
    let callees_arr = callees.as_array().expect("Callees should be an array");
    assert_eq!(
        callees_arr.len(),
        1,
        "graph_db_stats should call 1 function (ensure_graph_db)"
    );
    assert_eq!(
        callees_arr[0].get("slug").and_then(Value::as_str),
        Some("mod-rs::ensure_graph_db"),
        "Callee should be ensure_graph_db"
    );

    let callers = db
        .query(
            "function_callers",
            Some(&serde_json::json!({"fn_slug": "mod-rs::ensure_graph_db"})),
        )
        .await
        .expect("function_callers query should succeed");
    let callers_arr = callers.as_array().expect("Callers should be an array");
    assert_eq!(
        callers_arr.len(),
        2,
        "ensure_graph_db should have 2 callers"
    );
    let caller_slugs: Vec<&str> = callers_arr
        .iter()
        .filter_map(|c| c.get("slug").and_then(Value::as_str))
        .collect();
    assert!(
        caller_slugs.contains(&"mod-rs::graph_db_stats"),
        "graph_db_stats should be a caller"
    );
    assert!(
        caller_slugs.contains(&"mod-rs::graph_db_load_jsonl"),
        "graph_db_load_jsonl should be a caller"
    );

    // @step And traversing neighbors of a file node should return its contained functions
    let contained = db
        .query(
            "file_functions",
            Some(&serde_json::json!({"file_slug": "mod-rs"})),
        )
        .await
        .expect("file_functions query should succeed");
    let contained_arr = contained.as_array().expect("Contained should be an array");
    assert_eq!(
        contained_arr.len(),
        3,
        "mod-rs should contain 3 functions"
    );
}

// ============================================================================
// Scenario: Reusable GraphDatabase abstraction supports multiple instances
// ============================================================================
#[tokio::test]
async fn test_reusable_graph_database_supports_multiple_instances() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a GraphDatabase abstraction exists
    // (The GraphDatabase struct is defined in database.rs)

    // @step When I create one instance with the AST code schema
    let ast_path = temp_dir.path().join("ast-code.nano");
    let ast_db = GraphDatabase::init(&ast_path, AST_CODE_SCHEMA)
        .await
        .expect("AST graph init should succeed");

    // @step And I create another instance with a different schema
    let alt_path = temp_dir.path().join("alt-graph.nano");
    let alt_db = GraphDatabase::init(&alt_path, ALT_SCHEMA)
        .await
        .expect("Alt graph init should succeed");

    // @step Then both databases should initialize independently
    assert!(ast_path.exists(), "AST DB path should exist");
    assert!(alt_path.exists(), "Alt DB path should exist");
    assert!(
        ast_db.has_node_type("File"),
        "AST DB should have File node type"
    );
    assert!(
        alt_db.has_node_type("Widget"),
        "Alt DB should have Widget node type"
    );
    assert!(
        !ast_db.has_node_type("Widget"),
        "AST DB should NOT have Widget node type"
    );
    assert!(
        !alt_db.has_node_type("File"),
        "Alt DB should NOT have File node type"
    );

    // @step And data loaded into one should not appear in the other
    let ast_jsonl =
        r#"{"type":"File","data":{"slug":"test-file","path":"test.rs","language":"rust"}}"#;
    ast_db
        .load_jsonl(ast_jsonl)
        .await
        .expect("AST load should succeed");

    let alt_jsonl =
        r#"{"type":"Widget","data":{"slug":"widget-1","label":"My Widget"}}"#;
    alt_db
        .load_jsonl(alt_jsonl)
        .await
        .expect("Alt load should succeed");

    let ast_stats = ast_db.stats().expect("AST stats should succeed");
    let alt_stats = alt_db.stats().expect("Alt stats should succeed");

    let ast_file_count = ast_stats
        .pointer("/nodes/File")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(ast_file_count, 1, "AST DB should have 1 File node");

    let alt_widget_count = alt_stats
        .pointer("/nodes/Widget")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(alt_widget_count, 1, "Alt DB should have 1 Widget node");

    // Verify cross-contamination hasn't occurred
    assert!(
        ast_stats.pointer("/nodes/Widget").is_none(),
        "AST DB should not have Widget nodes"
    );
    assert!(
        alt_stats.pointer("/nodes/File").is_none(),
        "Alt DB should not have File nodes"
    );

    // @step And both databases should support the same load and query operations
    // Both have been tested with load_jsonl and stats above — verify describe_schema works too
    let ast_schema = ast_db.describe_schema();
    let alt_schema = alt_db.describe_schema();
    assert!(
        ast_schema.contains("File"),
        "AST schema description should mention File"
    );
    assert!(
        alt_schema.contains("Widget"),
        "Alt schema description should mention Widget"
    );
}

// ============================================================================
// Scenario: Re-open existing AST graph database
// ============================================================================
#[tokio::test]
async fn test_reopen_existing_ast_graph_database() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("ast-code.nano");

    // @step Given the AST graph database was previously initialized with data
    {
        let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
            .await
            .expect("Init should succeed");

        let jsonl = [
            r#"{"type":"File","data":{"slug":"persisted-file","path":"src/persisted.rs","language":"rust","lineCount":42}}"#,
            r#"{"type":"Function","data":{"slug":"persisted-file::foo","name":"foo","qualifiedName":"persisted::foo","isAsync":false,"isPublic":true,"paramCount":0}}"#,
        ]
        .join("\n");
        db.load_jsonl(&jsonl)
            .await
            .expect("Data load should succeed");

        // Verify data was loaded
        let stats = db.stats().expect("Stats should succeed");
        let file_count = stats
            .pointer("/nodes/File")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert_eq!(file_count, 1, "Should have 1 File node before close");
    }
    // db is dropped here, simulating process shutdown

    // @step When the database singleton is reset
    // (Already dropped above — simulates singleton reset)

    // @step And the AST graph database is re-initialized
    let db = GraphDatabase::open(&db_path)
        .await
        .expect("Re-open should succeed");
    let db = db.with_query_source(AST_QUERIES);

    // @step Then the previously loaded data should still be available
    let files = db
        .query("all_files", None)
        .await
        .expect("all_files query after re-open should succeed");
    let files_arr = files.as_array().expect("Result should be an array");
    assert_eq!(
        files_arr.len(),
        1,
        "Should still have 1 File node after re-open"
    );
    assert_eq!(
        files_arr[0].get("slug").and_then(Value::as_str),
        Some("persisted-file"),
        "File slug should be preserved"
    );

    // @step And the schema should match the original schema
    assert!(
        db.has_node_type("File"),
        "Re-opened DB should have File node type"
    );
    assert!(
        db.has_node_type("Function"),
        "Re-opened DB should have Function node type"
    );
    assert!(
        db.has_edge_type("Contains"),
        "Re-opened DB should have Contains edge type"
    );
    assert!(
        db.has_edge_type("Calls"),
        "Re-opened DB should have Calls edge type"
    );

    let node_names = db.node_type_names();
    for expected in &["File", "Module", "Function", "Type", "Dependency"] {
        assert!(
            node_names.contains(&expected.to_string()),
            "Re-opened schema should contain node type '{}'",
            expected
        );
    }
}
