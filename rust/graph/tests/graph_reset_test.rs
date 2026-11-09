// Feature: spec/features/graph-database-reset.feature
//
// Tests for graph database reset functionality — force-rebuild after schema changes.
// Validates that the reset flag clears both on-disk and in-memory graph state.
//
// RPC-092: lifted verbatim from rust/napi/src/graph/graph_reset_tests.rs to a
// standalone integration test against the new codelet-graph crate. The
// scenario-level coverage anchor for the RPC-092 feature file lives in
// rust/graph/tests/codelet_graph_crate_lift.rs (which shells out to this
// binary for the "lifted graph reset" scenario).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use std::path::PathBuf;

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::database::GraphDatabase;
use codelet_graph::registry;

/// Minimal valid schema for testing.
const TEST_SCHEMA: &str = r#"
node File {
path: String @key
}
"#;

/// Modified schema with a new node type (simulates schema change).
const TEST_SCHEMA_V2: &str = r#"
node File {
path: String @key
}
node Type {
slug: String @key
typeKind: String?
}
"#;

/// Create a temporary database directory path.
fn temp_db_path(suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fspec-graph-reset-test-{}-{}",
        suffix,
        std::process::id()
    ));
    // Clean up any leftover from previous runs
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// KGRAPH-073: a temp project root holding one Python function `greet`.
fn temp_project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("project");
    std::fs::create_dir_all(root.join("src")).expect("src dir");
    std::fs::write(root.join("src/main.py"), "def greet():\n    return 1\n").expect("source");
    let root = root.canonicalize().expect("canonical root");
    (dir, root)
}

fn index_ok(output: &str) -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(output).expect("index output is JSON");
    assert!(value.get("error").is_none(), "index failed: {output}");
    value
}

// ============================================================================
// Scenario: Reset flag deletes on-disk database and re-indexes with fresh schema
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reset_deletes_on_disk_database() {
    let (_dir, root) = temp_project();
    let db_path = registry::code_graph_path(&root);

    // @step Given an existing AST graph database at "<root>/.fspec/graph/ast-code.nano/"
    let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
        .await
        .expect("init should succeed");
    assert!(
        db_path.join("schema.ir.json").exists(),
        "schema.ir.json should exist"
    );

    // @step And the compiled schema has a new enum value not present in the on-disk schema
    assert!(!registry::code_graph_available(&root));

    // @step When I run ast_index with reset set to true
    let output = dispatch_ast_index(&root, true, false).await;

    // @step Then the on-disk ".nano" directory is deleted before re-indexing
    let on_disk = std::fs::read_to_string(db_path.join("schema.pg")).expect("schema.pg");
    assert_eq!(on_disk, registry::AST_CODE_SCHEMA);

    // @step And a fresh database is initialized with the compiled schema
    assert!(registry::code_graph_available(&root));

    // @step And the index completes successfully with entity counts
    let result = index_ok(&output);
    assert!(result["entities_loaded"].as_u64() > Some(0), "{output}");
}

// ============================================================================
// Scenario: Reset flag clears in-memory graph singleton so fresh schema takes effect
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reset_clears_in_memory_registry() {
    let (_dir, root) = temp_project();

    // @step Given an AST graph is cached in the in-memory registry
    index_ok(&dispatch_ast_index(&root, false, false).await);
    drop(registry::read_code_graph(&root).await.expect("graph opens"));

    // @step And the on-disk database has been manually deleted
    std::fs::remove_dir_all(registry::code_graph_path(&root)).expect("delete graph");

    // @step When I run ast_index with reset set to true
    let output = dispatch_ast_index(&root, true, false).await;

    // @step Then the graph held in memory for that root is dropped
    index_ok(&output);

    // @step And the next access opens the graph with the compiled schema
    assert!(registry::code_graph_available(&root));
    let graph = registry::read_code_graph(&root).await.expect("graph opens");

    // @step And subsequent ast_search queries return results without process restart
    let search = codelet_graph::ast_dispatch::dispatch_ast_search(
        &graph,
        "greet",
        Some("Function"),
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    let search: serde_json::Value = serde_json::from_str(&search).expect("search JSON");
    assert_eq!(search["count"], 1, "{search}");
}

// ============================================================================
// Scenario: Schema mismatch without reset flag returns actionable error
// ============================================================================
#[tokio::test]
async fn test_schema_mismatch_returns_actionable_error() {
    let db_path = temp_db_path("schema-mismatch");

    // @step Given an existing AST graph database created with an older schema
    let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
        .await
        .expect("init should succeed");

    // @step And the compiled schema has changed since the database was created
    // (TEST_SCHEMA_V2 differs from TEST_SCHEMA)

    // @step When I run ast_index without the reset flag
    let result: Result<GraphDatabase, String> =
        GraphDatabase::open_or_init_with_schema_check(&db_path, TEST_SCHEMA_V2).await;

    // @step Then the error message includes "Schema has changed"
    let err = result.expect_err("Should fail with schema mismatch");
    assert!(
        err.contains("Schema has changed"),
        "Error should mention schema change, got: {err}"
    );

    // @step And the error message tells the user to re-index with reset set to true
    assert!(
        err.contains("reset"),
        "Error should mention reset flag, got: {err}"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&db_path);
}

/// AST query source for executing named queries.
const AST_QUERY_SOURCE: &str = include_str!("../schemas/ast-queries.gq");

// ============================================================================
// Scenario: Queries work immediately after reset and re-index
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_queries_work_after_reset_and_reindex() {
    // @step Given an AST graph database has been reset and re-indexed
    let (_dir, root) = temp_project();
    let _old = GraphDatabase::init(&registry::code_graph_path(&root), TEST_SCHEMA)
        .await
        .expect("init should succeed");
    drop(_old);
    index_ok(&dispatch_ast_index(&root, true, false).await);
    let db = registry::read_code_graph(&root).await.expect("graph opens");

    // @step When I run ast_search with query "function"
    let search_result = db
        .query_with_source(AST_QUERY_SOURCE, "all_functions", None)
        .await
        .expect("all_functions query should succeed after reset");

    // @step Then results are returned from the freshly indexed graph
    let functions = search_result.as_array().expect("should be array");
    assert!(
        !functions.is_empty(),
        "all_functions should return loaded entities"
    );
    assert!(
        functions
            .iter()
            .any(|f| f.get("name").and_then(|v| v.as_str()) == Some("greet")),
        "Should find the indexed 'greet' function"
    );

    // @step When I run ast_neighbors with a valid node_id from the new index
    let params = serde_json::json!({ "slug": "src/main.py" });
    let neighbors_result = db
        .query_with_source(AST_QUERY_SOURCE, "file_functions", Some(&params))
        .await
        .expect("file_functions neighbor query should succeed after reset");

    // @step Then neighbors are returned successfully
    let neighbors = neighbors_result.as_array().expect("should be array");
    assert!(
        !neighbors.is_empty(),
        "file_functions should return the contained function"
    );

    // @step When I run ast_dead_code
    let dead_code_result = db
        .query_with_source(AST_QUERY_SOURCE, "orphan_files", None)
        .await;

    // @step Then dead code analysis completes without schema errors
    assert!(
        dead_code_result.is_ok(),
        "orphan_files query should execute without schema errors: {:?}",
        dead_code_result.err()
    );
}

// ============================================================================
// Additional: AstIndex deserializes with reset flag
// ============================================================================
#[test]
fn test_ast_index_deserializes_with_reset_flag() {
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Given a JSON payload for ast_index with reset set to true
    let json = r#"{"action_type":"ast_index","reset":true}"#;

    // @step When the action is deserialized
    let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

    // @step Then it should succeed with reset set to true
    assert!(
        result.is_ok(),
        "AstIndex should parse with reset flag: {:?}",
        result.err()
    );
    if let Ok(GraphSearchAction::AstIndex { path, reset, .. }) = result {
        assert!(path.is_none(), "Path should be None");
        assert_eq!(reset, Some(true), "Reset should be Some(true)");
    } else {
        panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
    }
}

// ============================================================================
// Additional: AstIndex backwards compatible without reset flag
// ============================================================================
#[test]
fn test_ast_index_backwards_compatible_without_reset() {
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Given a JSON payload for ast_index without reset field
    let json = r#"{"action_type":"ast_index"}"#;

    // @step When the action is deserialized
    let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

    // @step Then it should succeed with reset as None (backwards compatible)
    assert!(
        result.is_ok(),
        "AstIndex should parse without reset: {:?}",
        result.err()
    );
    if let Ok(GraphSearchAction::AstIndex { reset, .. }) = result {
        assert!(reset.is_none(), "Reset should be None when omitted");
    } else {
        panic!("Expected AstIndex variant, got: {:?}", result.unwrap());
    }
}

// ============================================================================
// Additional: a reset of a root without data reports that nothing was deleted
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reset_returns_false_when_no_data() {
    let (_dir, root) = temp_project();
    let mut lease = registry::write_code_graph(&root)
        .await
        .expect("write lease");
    assert_eq!(
        lease.reset(),
        Ok(false),
        "Should return false when no data exists"
    );
}
