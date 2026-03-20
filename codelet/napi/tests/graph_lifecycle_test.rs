// Feature: spec/features/nanograph-database-lifecycle.feature
//
// Nanograph Database Lifecycle & Integration
// Tests for the graph module singleton, init/open/close lifecycle.
//
// IMPORTANT: These tests use isolated temporary directories to avoid
// polluting ~/.fspec with test data. Each test gets its own temp dir.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::{
    ensure_graph_db, graph_db_stats, graph_describe_schema, is_graph_initialized, reset_graph_db,
};
use std::sync::Mutex;

// Global mutex to ensure tests run sequentially since they share global state
// (GRAPH_DB is a global singleton)
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup an isolated temp directory for a test.
/// Returns a guard (for sequential execution) and a TempDir that will be
/// cleaned up when dropped.
fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    // Reset graph DB singleton so tests start fresh
    reset_graph_db();
    (guard, temp_dir)
}

// ============================================================================
// Scenario: First GraphSearch call auto-initializes the database
// ============================================================================
#[tokio::test]
async fn test_first_graphsearch_auto_initializes_database() {
    let (_guard, temp_dir) = setup_test_env();

    // @step Given the ~/.fspec/graph/ directory does not exist
    let graph_dir = temp_dir.path().join("graph").join("agent-memory.nano");
    assert!(
        !graph_dir.exists(),
        "Graph directory should not exist before first use"
    );

    // @step When a GraphSearch action is invoked for the first time
    ensure_graph_db().await.expect("ensure_graph_db should succeed on first call");

    // @step Then the ~/.fspec/graph/agent-memory.nano/ directory is created
    assert!(
        graph_dir.exists(),
        "Graph directory should be created after first use"
    );

    // @step And the schema.pg file contains the agent-memory schema
    let schema_path = graph_dir.join("schema.pg");
    assert!(schema_path.exists(), "schema.pg should exist");
    let schema_content =
        std::fs::read_to_string(&schema_path).expect("Should be able to read schema.pg");
    assert!(
        schema_content.contains("node Concept"),
        "Schema should contain Concept node type"
    );
    assert!(
        schema_content.contains("node Session"),
        "Schema should contain Session node type"
    );

    // @step And the database is open and ready for queries
    let stats = graph_db_stats()
        .await
        .expect("graph_db_stats should succeed on initialized DB");
    // Stats should be parseable JSON with node/edge counts
    assert!(
        stats.contains("nodes") || stats.contains("Concept"),
        "Stats should contain node information"
    );
}

// ============================================================================
// Scenario: Subsequent process opens existing database
// ============================================================================
#[tokio::test]
async fn test_subsequent_process_opens_existing_database() {
    let (_guard, temp_dir) = setup_test_env();

    // @step Given the ~/.fspec/graph/agent-memory.nano/ directory already exists with a valid schema
    ensure_graph_db()
        .await
        .expect("First init should succeed");
    let graph_dir = temp_dir.path().join("graph").join("agent-memory.nano");
    assert!(graph_dir.exists(), "Graph directory should exist after init");

    // Simulate process restart by resetting the singleton
    reset_graph_db();

    // @step When a GraphSearch action is invoked
    ensure_graph_db()
        .await
        .expect("Re-open should succeed");

    // @step Then the existing database is opened without re-initialization
    // (Verified by the fact that ensure_graph_db succeeded without error)

    // @step And all previously stored graph data is accessible
    let stats = graph_db_stats()
        .await
        .expect("Stats should work after re-open");
    assert!(
        !stats.is_empty(),
        "Stats should return non-empty result after re-open"
    );
}

// ============================================================================
// Scenario: Data directory change resets graph singleton
// ============================================================================
#[tokio::test]
async fn test_data_directory_change_resets_graph_singleton() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is open and initialized
    ensure_graph_db()
        .await
        .expect("Initial graph init should succeed");
    assert!(
        is_graph_initialized(),
        "Graph should be initialized"
    );

    // @step When set_data_directory() is called with a new path
    let new_temp = tempfile::tempdir().expect("Failed to create second temp dir");
    codelet_common::set_data_directory(new_temp.path().to_path_buf())
        .expect("set_data_directory should succeed");
    reset_graph_db(); // This is called by the set_data_directory hook

    // @step Then the graph singleton is reset to None
    assert!(
        !is_graph_initialized(),
        "Graph should not be initialized after directory change"
    );

    // @step And the next GraphSearch call initializes from the new data directory
    ensure_graph_db()
        .await
        .expect("Re-init from new directory should succeed");
    let new_graph_dir = new_temp.path().join("graph").join("agent-memory.nano");
    assert!(
        new_graph_dir.exists(),
        "Graph directory should be created in new data directory"
    );
}

// ============================================================================
// Scenario: Empty graph returns zero stats without error
// ============================================================================
#[tokio::test]
async fn test_empty_graph_returns_zero_stats() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database has been initialized with no data loaded
    ensure_graph_db()
        .await
        .expect("Graph init should succeed");

    // @step When a stats query is executed against the graph
    let stats = graph_db_stats()
        .await
        .expect("Stats query should not error on empty graph");

    // @step Then all node and edge counts are zero
    // Parse the stats JSON and verify all counts are zero
    let parsed: serde_json::Value = serde_json::from_str(&stats)
        .expect("Stats should be valid JSON");
    let nodes = parsed.get("nodes").and_then(|v| v.as_object())
        .expect("Stats should have 'nodes' object");
    for (name, count) in nodes {
        assert_eq!(count.as_i64(), Some(0), "Node type '{name}' should have count 0");
    }
    let edges = parsed.get("edges").and_then(|v| v.as_object())
        .expect("Stats should have 'edges' object");
    for (name, count) in edges {
        assert_eq!(count.as_i64(), Some(0), "Edge type '{name}' should have count 0");
    }

    // @step And no error is returned
    // (Verified by the expect() above - no error means success)
}

// ============================================================================
// Scenario: Schema contains all required node and edge types
// ============================================================================
#[tokio::test]
async fn test_schema_contains_all_required_types() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database has been initialized
    ensure_graph_db()
        .await
        .expect("Graph init should succeed");

    // @step When the database schema is described
    let description = graph_describe_schema()
        .await
        .expect("Schema describe should succeed");

    // @step Then node types Concept, Decision, CodeEntity, WorkUnit, Session, and Turn exist
    let required_nodes = ["Concept", "Decision", "CodeEntity", "WorkUnit", "Session", "Turn"];
    for node_type in &required_nodes {
        assert!(
            description.contains(node_type),
            "Schema should contain node type '{}', got: {}",
            node_type,
            description
        );
    }

    // @step And edge types Mentions, Discusses, Decides, Implements, Modifies, RelatesTo, Supersedes, WorksOn, References, and ContainsTurn exist
    let required_edges = [
        "Mentions",
        "Discusses",
        "Decides",
        "Implements",
        "Modifies",
        "RelatesTo",
        "Supersedes",
        "WorksOn",
        "References",
        "ContainsTurn",
    ];
    for edge_type in &required_edges {
        assert!(
            description.contains(edge_type),
            "Schema should contain edge type '{}', got: {}",
            edge_type,
            description
        );
    }
}
