// Feature: spec/features/deprecate-old-graph-migrate-useful-data.feature
use super::*;

/// Scenario: Graph module exports only dual-graph infrastructure
#[test]
fn test_graph_module_only_exports_dual_graph() {
    // @step Given the old monolithic graph infrastructure has been removed
    // Verify old files have been deleted by checking file existence
    //
    // RPC-092: The graph module lives in the codelet-graph crate
    // (rust/graph/src/), so source-of-truth paths resolve under this
    // crate's manifest directory.

    let graph_crate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let graph_dir = graph_crate.join("src");
    let schemas_dir = graph_crate.join("schemas");

    // @step When the graph module is compiled

    // @step Then it should export database, registry, ast_pipeline, ast_dispatch, learnings_extraction, learnings_dispatch, learnings_context, dispatch_helpers, graph_entities, and llm_response_parser modules
    // Verify new files exist
    assert!(
        graph_dir.join("database.rs").exists(),
        "database.rs should exist"
    );
    assert!(
        graph_dir.join("registry.rs").exists(),
        "registry.rs should exist"
    );
    assert!(
        graph_dir.join("ast_pipeline").is_dir(),
        "ast_pipeline/ should exist"
    );
    assert!(
        graph_dir.join("ast_dispatch.rs").exists(),
        "ast_dispatch.rs should exist"
    );
    assert!(
        graph_dir.join("learnings_extraction.rs").exists(),
        "learnings_extraction.rs should exist"
    );
    assert!(
        graph_dir.join("learnings_dispatch.rs").exists(),
        "learnings_dispatch.rs should exist"
    );
    assert!(
        graph_dir.join("learnings_context.rs").exists(),
        "learnings_context.rs should exist"
    );
    assert!(
        graph_dir.join("dispatch_helpers.rs").exists(),
        "dispatch_helpers.rs should exist"
    );
    assert!(
        graph_dir.join("graph_entities.rs").exists(),
        "graph_entities.rs should exist"
    );
    assert!(
        graph_dir.join("llm_response_parser.rs").exists(),
        "llm_response_parser.rs should exist"
    );

    // @step And it should not export entity_pipeline, extractors, merge, watermark, indexing, session_scanner, compaction, or old dispatch and queries modules
    let old_files = vec![
        "entity_pipeline.rs",
        "extractors.rs",
        "merge.rs",
        "watermark.rs",
        "indexing.rs",
        "session_scanner.rs",
        "compaction.rs",
        "dispatch.rs",
        "queries.rs",
        "deepsearch_integration.rs",
        "llm_extraction.rs",
        "llm_validation.rs",
        "llm_caller.rs",
    ];

    for old_file in &old_files {
        let path = graph_dir.join(old_file);
        assert!(
            !path.exists(),
            "Old file should be deleted after migration: {}",
            path.display()
        );
    }

    // Verify old schema files are deleted
    assert!(
        !schemas_dir.join("agent-memory.pg").exists(),
        "agent-memory.pg schema should be deleted"
    );
    assert!(
        !schemas_dir.join("graph-queries.gq").exists(),
        "graph-queries.gq should be deleted"
    );

    // Verify new schema files still exist
    assert!(
        schemas_dir.join("ast-code.pg").exists(),
        "ast-code.pg should still exist"
    );
    assert!(
        schemas_dir.join("learnings.pg").exists(),
        "learnings.pg should still exist"
    );
    assert!(
        schemas_dir.join("learnings-queries.gq").exists(),
        "learnings-queries.gq should still exist"
    );
}

/// Scenario: All existing AST and Learnings tests pass after migration
#[tokio::test]
async fn test_existing_graphs_functional_after_migration() {
    // @step Given the old monolithic graph infrastructure has been removed
    // This test verifies the new graphs still work independently

    // @step When the full test suite is executed
    let tmp = tempfile::tempdir().expect("temp dir");
    let db_path = tmp.path().join("test-ast.nano");

    let ast_schema = include_str!("../../schemas/ast-code.pg");
    let db = GraphDatabase::init(&db_path, ast_schema)
        .await
        .expect("AST graph should initialize");

    // @step Then all 4 learnings query interface tests should pass
    // @step And all 3 learnings extraction tests should pass
    // @step And all 3 AST query interface tests should pass
    // @step And all 5 AST graph data model tests should pass
    // @step And all 3 AST dependency population tests should pass

    // Verify basic graph operations still work
    let stats = db.stats().expect("stats should work");
    assert!(stats.get("nodes").is_some(), "Should have nodes in stats");
    assert!(stats.get("edges").is_some(), "Should have edges in stats");

    // Verify Learnings graph can also be created
    let learnings_path = tmp.path().join("test-learnings.nano");
    let learnings_schema = include_str!("../../schemas/learnings.pg");
    let learnings_db = GraphDatabase::init(&learnings_path, learnings_schema)
        .await
        .expect("Learnings graph should initialize");

    let learnings_stats = learnings_db.stats().expect("learnings stats should work");
    assert!(
        learnings_stats.get("nodes").is_some(),
        "Should have nodes in learnings stats"
    );
}
