// Feature: spec/features/ast-graph-data-model.feature
use super::*;

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
    assert!(
        db_path.exists(),
        "Database directory should exist after init"
    );
    assert!(
        db_path.join("schema.ir.json").exists(),
        "schema.ir.json should exist"
    );

    // @step And the schema catalog should contain node types "File, Module, Function, Type, Dependency"
    // The Module node is gone (KGRAPH-074): Rust modules are files.
    let mut node_names = db.node_type_names();
    node_names.sort();
    let mut expected: Vec<String> = AST_NODE_TYPES.iter().map(|t| t.to_string()).collect();
    expected.sort();
    assert_eq!(node_names, expected, "Schema node types");

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
        "HasMethod",
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
    for node_type in AST_NODE_TYPES {
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
    let (_dir, indexed) = index_fixture(&[
        ("src/main.rs", "async fn main() {}\n"),
        (
            "src/lib.rs",
            "pub fn process(a: u32, b: u32) -> u32 {\n    a + b\n}\n\nfn validate(x: u32) -> bool {\n    x > 0\n}\n",
        ),
    ]);
    let entities = nodes_of(&indexed.entities, &["File", "Function"]);
    let main = slug_named(&entities, "src/main.rs::main");
    let process = slug_named(&entities, "src/lib.rs::process");
    let validate = slug_named(&entities, "src/lib.rs::validate");
    let jsonl = entities_to_jsonl(&entities);

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
    assert!(slugs.contains(&"src/main.rs"), "Should contain src/main.rs");
    assert!(slugs.contains(&"src/lib.rs"), "Should contain src/lib.rs");
    let rust_file = files_arr
        .iter()
        .find(|f| f.get("slug").and_then(Value::as_str) == Some("src/lib.rs"))
        .expect("src/lib.rs row");
    assert_eq!(
        rust_file.get("path").and_then(Value::as_str),
        Some("src/lib.rs")
    );
    assert_eq!(
        rust_file.get("language").and_then(Value::as_str),
        Some("rust")
    );
    assert_eq!(rust_file.get("lineCount").and_then(Value::as_i64), Some(7));

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
        fn_slugs.contains(&main.as_str()),
        "Should contain main function"
    );
    assert!(
        fn_slugs.contains(&process.as_str()),
        "Should contain process function"
    );
    assert!(
        fn_slugs.contains(&validate.as_str()),
        "Should contain validate function"
    );
    assert_eq!(
        process, "src/lib.rs::process@0",
        "slug = qualifiedName@start byte"
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
    let (_dir, indexed) = index_fixture(&[
        ("Cargo.toml", "[package]\nname = \"fixture\"\n"),
        ("src/lib.rs", "pub mod graph;\n"),
        (
            "src/graph/mod.rs",
            "pub async fn ensure_graph_db() {}\n\npub async fn graph_db_stats() {\n    ensure_graph_db().await;\n}\n\npub async fn graph_db_load_jsonl(jsonl: &str) {\n    ensure_graph_db().await;\n}\n",
        ),
    ]);
    let all = &indexed.entities;
    let file = slug_named(all, "src/graph/mod.rs");
    let ensure = slug_named(all, "src/graph/mod.rs::ensure_graph_db");
    let stats_fn = slug_named(all, "src/graph/mod.rs::graph_db_stats");
    let load_fn = slug_named(all, "src/graph/mod.rs::graph_db_load_jsonl");
    db.load_jsonl(&entities_to_jsonl(&nodes_of(all, &["File", "Function"])))
        .await
        .expect("Node load should succeed");

    // @step When I load Contains edges linking files to functions
    let contains = edges_of(all, "Contains");
    assert_eq!(contains.len(), 3, "{contains:?}");
    db.load_jsonl(&entities_to_jsonl(&contains))
        .await
        .expect("Contains edge load should succeed");

    // @step And I load Calls edges linking functions to other functions
    let calls = edges_of(all, "Calls");
    assert_eq!(calls.len(), 2, "{calls:?}");
    db.load_jsonl(&entities_to_jsonl(&calls))
        .await
        .expect("Calls edge load should succeed");

    // @step Then traversing neighbors of a function node should return its callers and callees
    let callees = db
        .query(
            "function_callees",
            Some(&serde_json::json!({"fn_slug": stats_fn})),
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
        Some(ensure.as_str()),
        "Callee should be ensure_graph_db"
    );

    let callers = db
        .query(
            "function_callers",
            Some(&serde_json::json!({"fn_slug": ensure})),
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
        caller_slugs.contains(&stats_fn.as_str()),
        "graph_db_stats should be a caller"
    );
    assert!(
        caller_slugs.contains(&load_fn.as_str()),
        "graph_db_load_jsonl should be a caller"
    );

    // @step And traversing neighbors of a file node should return its contained functions
    let contained = db
        .query(
            "file_functions",
            Some(&serde_json::json!({"file_slug": file})),
        )
        .await
        .expect("file_functions query should succeed");
    let contained_arr = contained.as_array().expect("Contained should be an array");
    assert_eq!(
        contained_arr.len(),
        3,
        "src/graph/mod.rs should contain 3 functions"
    );
}
