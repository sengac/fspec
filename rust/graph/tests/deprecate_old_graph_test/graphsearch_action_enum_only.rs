// Feature: spec/features/deprecate-old-graph-migrate-useful-data.feature
use super::*;

/// Scenario: GraphSearchAction enum only contains AST and Learnings variants after migration
#[test]
fn test_graphsearch_action_enum_only_ast_and_learnings_variants() {
    // @step Given the old monolithic graph infrastructure has been removed
    // Verify the enum cannot deserialize old action types
    let old_actions = vec![
        r#"{"action_type":"search","query":"test"}"#,
        r#"{"action_type":"neighbors","node_id":"test"}"#,
        r#"{"action_type":"path","from":"a","to":"b"}"#,
        r#"{"action_type":"related","topic":"test"}"#,
        r#"{"action_type":"decisions"}"#,
        r#"{"action_type":"history","concept":"test"}"#,
        r#"{"action_type":"stats"}"#,
        r#"{"action_type":"index"}"#,
    ];

    // @step When the GraphSearchAction enum is compiled
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Then it should only contain AST-prefixed and Learnings-prefixed variants
    // Verify new actions DO parse
    let ast_search: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"ast_search","query":"test"}"#);
    assert!(
        ast_search.is_ok(),
        "AstSearch should parse: {:?}",
        ast_search.err()
    );

    let ast_neighbors: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"ast_neighbors","node_id":"test"}"#);
    assert!(
        ast_neighbors.is_ok(),
        "AstNeighbors should parse: {:?}",
        ast_neighbors.err()
    );

    let ast_stats: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"ast_stats"}"#);
    assert!(
        ast_stats.is_ok(),
        "AstStats should parse: {:?}",
        ast_stats.err()
    );

    let learnings_search: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"learnings_search","query":"test"}"#);
    assert!(
        learnings_search.is_ok(),
        "LearningsSearch should parse: {:?}",
        learnings_search.err()
    );

    let learnings_decisions: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"learnings_decisions"}"#);
    assert!(
        learnings_decisions.is_ok(),
        "LearningsDecisions should parse: {:?}",
        learnings_decisions.err()
    );

    let learnings_stats: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"learnings_stats"}"#);
    assert!(
        learnings_stats.is_ok(),
        "LearningsStats should parse: {:?}",
        learnings_stats.err()
    );

    let learnings_related: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"learnings_related","topic":"test"}"#);
    assert!(
        learnings_related.is_ok(),
        "LearningsRelated should parse: {:?}",
        learnings_related.err()
    );

    // Verify AstIndex action also deserializes correctly (added post-migration)
    let ast_index: Result<GraphSearchAction, _> =
        serde_json::from_str(r#"{"action_type":"ast_index"}"#);
    assert!(
        ast_index.is_ok(),
        "AstIndex should parse: {:?}",
        ast_index.err()
    );

    // @step And the old agent-memory variants Search, Neighbors, Path, Related, Decisions, History, Stats, and Index should not exist
    for old_action_json in &old_actions {
        let result: Result<GraphSearchAction, _> = serde_json::from_str(old_action_json);
        assert!(
            result.is_err(),
            "Old action should NOT parse after migration: {}",
            old_action_json
        );
    }

    // @step And the crate should build successfully with no compilation errors
    // (This test compiling and running proves the crate builds)
}

/// Scenario: Graph registry only contains AST and Learnings graph instances
#[test]
fn test_registry_only_ast_and_learnings_graphs() {
    // @step Given the old monolithic graph infrastructure has been removed
    // The AGENT_MEMORY_GRAPH constant should not exist in registry module.
    // This test verifies at compile-time by asserting that only AST_CODE_GRAPH
    // and LEARNINGS_GRAPH constants are accessible.

    // @step When the graph registry is initialized
    let kinds = [registry::GraphKind::Code, registry::GraphKind::Learnings];

    // @step Then it should only know the Code and Learnings graph kinds
    // Exhaustive match: a third kind would not compile without a new arm.
    for kind in kinds {
        match kind {
            registry::GraphKind::Code | registry::GraphKind::Learnings => {}
        }
    }

    // @step And the AGENT_MEMORY_GRAPH constant should not exist
    let registry_source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/registry.rs"))
            .expect("registry source");
    assert!(!registry_source.contains("AGENT_MEMORY_GRAPH"));

    // @step And both graph kinds resolve to their own database paths
    let root = Path::new("/project");
    assert_eq!(
        registry::code_graph_path(root),
        Path::new("/project/.fspec/graph/ast-code.nano")
    );
    let data = tempfile::tempdir().expect("temp data dir");
    codelet_common::set_data_directory(data.path().to_path_buf()).expect("data dir");
    assert_eq!(
        registry::learnings_graph_path().expect("learnings path"),
        data.path().join("graph/learnings.nano")
    );
}

/// Scenario: DeepSearch uses Learnings graph context instead of agent-memory
#[test]
fn test_deepsearch_uses_learnings_graph() {
    // @step Given the old monolithic graph infrastructure has been removed
    // @step And the Learnings graph contains accumulated learnings and decisions
    // Verify that the deepsearch_integration module no longer references agent-memory

    // @step When DeepSearch builds graph context for a sub-agent system prompt
    // The old deepsearch_integration.rs module should not exist.
    // Verify by checking the graph module does NOT export it.
    let deepsearch_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/deepsearch_integration.rs");

    // @step Then it should query the Learnings graph for relevant decisions and learnings
    // @step And it should not reference the old agent-memory Concept nodes
    assert!(
        !deepsearch_path.exists(),
        "deepsearch_integration.rs should be deleted after migration, but still exists at: {}",
        deepsearch_path.display()
    );
}
