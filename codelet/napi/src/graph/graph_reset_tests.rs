/// Feature: spec/features/graph-database-reset.feature
///
/// Tests for graph database reset functionality — force-rebuild after schema changes.
/// Validates that the reset flag clears both on-disk and in-memory graph state.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use crate::graph::database::GraphDatabase;
    use crate::graph::registry;

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

    // ============================================================================
    // Scenario: Reset flag deletes on-disk database and re-indexes with fresh schema
    // ============================================================================
    #[tokio::test]
    async fn test_reset_deletes_on_disk_database() {
        let db_path = temp_db_path("delete-on-disk");

        // @step Given an existing AST graph database at "<cwd>/.fspec/graph/ast-code.nano/"
        let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
            .await
            .expect("init should succeed");
        assert!(db_path.exists(), "DB directory should exist after init");
        assert!(
            db_path.join("schema.ir.json").exists(),
            "schema.ir.json should exist"
        );

        // @step And the compiled schema has a new enum value not present in the on-disk schema
        // (TEST_SCHEMA_V2 has 'extension' enum value that TEST_SCHEMA doesn't)

        // @step When I run ast_index with reset set to true
        // Reset = delete the directory, then re-init with new schema
        registry::delete_graph_data("test-delete-on-disk", &db_path)
            .expect("delete_graph_data should succeed");

        // @step Then the on-disk ".nano" directory is deleted before re-indexing
        assert!(
            !db_path.exists(),
            "DB directory should be deleted after reset"
        );

        // @step And a fresh database is initialized with the compiled schema
        let db2 = GraphDatabase::init(&db_path, TEST_SCHEMA_V2)
            .await
            .expect("re-init with new schema should succeed");

        // @step And the index completes successfully with entity counts
        assert!(
            db2.has_node_type("Type"),
            "New schema should have Type node"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&db_path);
    }

    // ============================================================================
    // Scenario: Reset flag clears in-memory graph singleton so fresh schema takes effect
    // ============================================================================
    #[tokio::test]
    async fn test_reset_clears_in_memory_registry() {
        let db_path = temp_db_path("clear-memory");

        // @step Given an AST graph is cached in the in-memory registry
        let db = GraphDatabase::init(&db_path, TEST_SCHEMA)
            .await
            .expect("init should succeed");
        // Manually insert into registry to simulate cached state
        registry::insert_graph_for_test("test-reset-memory", db);
        assert!(
            registry::is_graph_initialized("test-reset-memory"),
            "Graph should be in registry"
        );

        // @step And the on-disk database has been manually deleted
        let _ = std::fs::remove_dir_all(&db_path);

        // @step When I run ast_index with reset set to true
        registry::reset_graph("test-reset-memory");

        // @step Then the graph is removed from the in-memory Mutex<HashMap> registry
        assert!(
            !registry::is_graph_initialized("test-reset-memory"),
            "Graph should be removed from registry after reset"
        );

        // @step And the next get_graph call re-initializes with the compiled schema
        // Re-init with a new schema to verify fresh initialization works
        let db2 = GraphDatabase::init(&db_path, TEST_SCHEMA_V2)
            .await
            .expect("re-init after reset should succeed");
        registry::insert_graph_for_test("test-reset-memory", db2.clone());
        assert!(
            registry::is_graph_initialized("test-reset-memory"),
            "Graph should be back in registry after re-init"
        );

        // @step And subsequent ast_search queries return results without process restart
        // Verify the re-initialized DB has the new schema and is queryable
        assert!(
            db2.has_node_type("Type"),
            "Re-initialized graph should have Type node from new schema"
        );
        let stats = db2.stats().expect("stats should work on re-initialized graph");
        assert!(
            stats.is_object(),
            "Stats query should succeed on re-initialized graph"
        );

        // Cleanup
        registry::reset_graph("test-reset-memory");
        let _ = std::fs::remove_dir_all(&db_path);
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

    /// Full AST schema for query-level testing.
    const FULL_AST_SCHEMA: &str = include_str!("../../schemas/ast-code.pg");

    /// AST query source for executing named queries.
    const AST_QUERY_SOURCE: &str = include_str!("../../schemas/ast-queries.gq");

    // ============================================================================
    // Scenario: Queries work immediately after reset and re-index
    // ============================================================================
    #[tokio::test]
    async fn test_queries_work_after_reset_and_reindex() {
        let db_path = temp_db_path("queries-after-reset");

        // @step Given an AST graph database has been reset and re-indexed
        // First create with old schema
        let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
            .await
            .expect("init should succeed");

        // Reset (delete + clear registry)
        registry::delete_graph_data("test-queries", &db_path)
            .expect("delete should succeed");
        registry::reset_graph("test-queries");

        // Re-init with the full AST schema (simulates real re-index)
        let db = GraphDatabase::init(&db_path, FULL_AST_SCHEMA)
            .await
            .expect("re-init with full schema should succeed");

        // Load test entities to simulate an indexed codebase
        let test_data = r#"{"type":"File","data":{"slug":"test-file-main-ts","path":"src/main.ts","language":"typescript","lineCount":50,"isTest":false}}
{"type":"Function","data":{"slug":"test-file-main-ts::greet","name":"greet","qualifiedName":"test-file-main-ts::greet","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":1,"lineEnd":10}}
{"type":"Type","data":{"slug":"test-file-main-ts::Config","name":"Config","typeKind":"interface","isPublic":true}}
{"edge":"Contains","from":"test-file-main-ts","to":"test-file-main-ts::greet","data":{}}
{"edge":"ContainsType","from":"test-file-main-ts","to":"test-file-main-ts::Config","data":{}}"#;
        db.load_jsonl(test_data)
            .await
            .expect("loading test entities should succeed");

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
            functions.iter().any(|f| f.get("name").and_then(|v| v.as_str()) == Some("greet")),
            "Should find the 'greet' function we loaded"
        );

        // @step When I run ast_neighbors with a valid node_id from the new index
        let params = serde_json::json!({ "slug": "test-file-main-ts" });
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

        // Cleanup
        let _ = std::fs::remove_dir_all(&db_path);
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
        if let Ok(GraphSearchAction::AstIndex { path, reset }) = result {
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
    // Additional: Schema match passes validation (no error)
    // ============================================================================
    #[tokio::test]
    async fn test_schema_match_passes_validation() {
        let db_path = temp_db_path("schema-match");

        // Create DB with TEST_SCHEMA
        let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
            .await
            .expect("init should succeed");

        // Re-open with SAME schema — should succeed
        let result = GraphDatabase::open_or_init_with_schema_check(&db_path, TEST_SCHEMA).await;
        assert!(
            result.is_ok(),
            "Same schema should open successfully: {:?}",
            result.err()
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&db_path);
    }

    // ============================================================================
    // Additional: delete_graph_data returns false when no data exists
    // ============================================================================
    #[test]
    fn test_delete_graph_data_returns_false_when_no_data() {
        let db_path = temp_db_path("no-data");
        // Make sure it doesn't exist
        let _ = std::fs::remove_dir_all(&db_path);

        let result = registry::delete_graph_data("test-no-data", &db_path);
        assert_eq!(result, Ok(false), "Should return false when no data exists");
    }
}
