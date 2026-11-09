// Feature: spec/features/ast-graph-data-model.feature
use super::*;

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

    let alt_jsonl = r#"{"type":"Widget","data":{"slug":"widget-1","label":"My Widget"}}"#;
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

    let mut node_names = db.node_type_names();
    node_names.sort();
    let mut expected: Vec<String> = AST_NODE_TYPES.iter().map(|t| t.to_string()).collect();
    expected.sort();
    assert_eq!(node_names, expected, "Re-opened schema node types");
    assert!(
        db.has_edge_type("HasMethod"),
        "Re-opened DB should have HasMethod edge type"
    );
}
