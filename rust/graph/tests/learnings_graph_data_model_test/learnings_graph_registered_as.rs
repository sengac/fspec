// Feature: spec/features/learnings-graph-data-model.feature
use super::*;

// ============================================================================
// Scenario: Learnings graph registered as named instance in registry
// ============================================================================
#[tokio::test]
async fn test_learnings_graph_registered_as_named_instance_in_registry() {
    let _guard = REGISTRY_TEST_MUTEX.lock().unwrap();

    // Set up isolated temp directory for the registry
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    registry::reset_all_graphs();

    // @step Given the GraphDatabase registry exists
    // (It's a lazy_static global — always exists)

    // @step When I request the Learnings graph from the registry
    let learnings_db = registry::read_learnings_graph()
        .await
        .expect("Getting learnings graph should succeed");

    // @step Then the registry should return a valid GraphDatabase instance
    assert!(
        learnings_db.has_node_type("Learning"),
        "Learnings DB should have Learning node type"
    );
    assert!(
        learnings_db.has_node_type("Exploration"),
        "Learnings DB should have Exploration node type"
    );

    // @step And the instance should be separate from the "ast-code" graph
    assert!(
        !learnings_db.has_node_type("File"),
        "Learnings DB should NOT have File node type from AST graph"
    );
    assert!(
        !learnings_db.has_edge_type("Contains"),
        "Learnings DB should NOT have Contains edge type from AST graph"
    );

    // @step And the database path should be under the global data directory
    let expected_path = temp_dir.path().join("graph/learnings.nano");
    assert_eq!(
        learnings_db.path(),
        expected_path.as_path(),
        "Learnings DB should be at the expected global path"
    );

    // Clean up
    drop(learnings_db);
    registry::reset_all_graphs();
}
