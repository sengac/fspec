// Feature: spec/features/portable-graph-bundles.feature
use super::*;

// ============================================================================
// Scenario: Export creates valid astbundle ZIP archive
// ============================================================================
#[tokio::test]
async fn test_export_creates_valid_astbundle_zip() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_export_db(temp_dir.path()).await;

    // @step Given a graph indexed with Function, File, Type, and Dependency nodes plus edges
    let stats = db.stats().expect("stats");
    let nodes = stats.get("nodes").expect("nodes");
    assert!(nodes.get("Function").and_then(|v| v.as_u64()).unwrap_or(0) > 0);
    assert!(nodes.get("File").and_then(|v| v.as_u64()).unwrap_or(0) > 0);

    // @step When I export the graph to an astbundle file
    let bundle_path = temp_dir.path().join("test.astbundle");
    db.export_bundle(&bundle_path, AST_CODE_SCHEMA)
        .await
        .expect("export bundle");

    // @step Then the output is a valid ZIP archive
    assert!(bundle_path.exists(), "bundle file should exist");
    let file = std::fs::File::open(&bundle_path).expect("open bundle");
    let mut archive = zip::ZipArchive::new(file).expect("valid ZIP");

    // @step And the archive contains entities.jsonl with all nodes and edges
    let mut entities_content = String::new();
    archive
        .by_name("entities.jsonl")
        .expect("entities.jsonl in archive")
        .read_to_string(&mut entities_content)
        .expect("read entities");
    let entity_lines: Vec<&str> = entities_content.lines().collect();
    // 6 nodes (2 files, 2 functions, 1 type, 1 dependency) + 5 edges = 11 entities
    assert!(
        entity_lines.len() >= 11,
        "expected at least 11 JSONL lines, got {}",
        entity_lines.len()
    );

    // @step And the archive contains metadata.json with version and entity counts
    let mut metadata_content = String::new();
    archive
        .by_name("metadata.json")
        .expect("metadata.json in archive")
        .read_to_string(&mut metadata_content)
        .expect("read metadata");
    let metadata: Value = serde_json::from_str(&metadata_content).expect("parse metadata");
    assert!(
        metadata.get("version").is_some(),
        "metadata should have version"
    );
    assert!(
        metadata.get("node_count").is_some(),
        "metadata should have node_count"
    );
    assert!(
        metadata.get("edge_count").is_some(),
        "metadata should have edge_count"
    );

    // @step And the archive contains schema.pg matching the current schema
    let mut schema_content = String::new();
    archive
        .by_name("schema.pg")
        .expect("schema.pg in archive")
        .read_to_string(&mut schema_content)
        .expect("read schema");
    assert_eq!(
        schema_content, AST_CODE_SCHEMA,
        "schema.pg should match current schema"
    );
}

// ============================================================================
// Scenario: Import loads bundle into empty graph with overwrite mode
// ============================================================================
#[tokio::test]
async fn test_import_loads_bundle_overwrite_mode() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_export_db(temp_dir.path()).await;

    // @step Given an exported astbundle file from a graph with known node and edge counts
    let original_stats = db.stats().expect("stats");
    let bundle_path = temp_dir.path().join("import-test.astbundle");
    db.export_bundle(&bundle_path, AST_CODE_SCHEMA)
        .await
        .expect("export bundle");

    // @step And an empty graph database
    let import_db_path = temp_dir.path().join("import-target.nano");
    let import_db = GraphDatabase::init(&import_db_path, AST_CODE_SCHEMA)
        .await
        .expect("init import DB");

    // @step When I import the bundle with overwrite mode
    import_db
        .import_bundle(&bundle_path, AST_CODE_SCHEMA, "overwrite")
        .await
        .expect("import bundle");

    // @step Then the graph contains the same number of nodes as the export source
    let import_stats = import_db.stats().expect("import stats");
    let orig_nodes = original_stats.get("nodes").expect("orig nodes");
    let imp_nodes = import_stats.get("nodes").expect("imp nodes");
    for key in ["Function", "File", "Type", "Dependency"] {
        assert_eq!(
            orig_nodes.get(key).and_then(|v| v.as_u64()),
            imp_nodes.get(key).and_then(|v| v.as_u64()),
            "node count mismatch for {key}"
        );
    }

    // @step And the graph contains the same number of edges as the export source
    let orig_edges = original_stats.get("edges").expect("orig edges");
    let imp_edges = import_stats.get("edges").expect("imp edges");
    for key in ["Contains", "ContainsType", "Imports", "DependsOn"] {
        assert_eq!(
            orig_edges.get(key).and_then(|v| v.as_u64()),
            imp_edges.get(key).and_then(|v| v.as_u64()),
            "edge count mismatch for {key}"
        );
    }
}

// ============================================================================
// Scenario: Import loads bundle with merge mode
// ============================================================================
#[tokio::test]
async fn test_import_loads_bundle_merge_mode() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given an exported astbundle file containing additional functions
    let source_db_path = temp_dir.path().join("source.nano");
    let source_db = GraphDatabase::init(&source_db_path, AST_CODE_SCHEMA)
        .await
        .expect("init source DB");
    let extra_jsonl = r#"{"type":"File","data":{"slug":"src-extra-ts","path":"src/extra.ts","language":"typescript","lineCount":50,"isTest":false}}
{"type":"Function","data":{"slug":"src-extra-ts::extraFn","name":"extraFn","qualifiedName":"src-extra-ts::extraFn","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":1,"lineEnd":5,"parameters":"","source":"function extraFn() {}","docstring":"","decorators":"","language":"typescript"}}
{"edge":"Contains","from":"src-extra-ts","to":"src-extra-ts::extraFn","data":{}}"#;
    source_db.load_jsonl(extra_jsonl).await.expect("load extra");
    let bundle_path = temp_dir.path().join("merge-test.astbundle");
    source_db
        .export_bundle(&bundle_path, AST_CODE_SCHEMA)
        .await
        .expect("export bundle");

    // @step And a graph with some existing functions
    let target_db_path = temp_dir.path().join("target.nano");
    let target_db = GraphDatabase::init(&target_db_path, AST_CODE_SCHEMA)
        .await
        .expect("init target DB");
    let existing_jsonl = r#"{"type":"File","data":{"slug":"src-existing-ts","path":"src/existing.ts","language":"typescript","lineCount":30,"isTest":false}}
{"type":"Function","data":{"slug":"src-existing-ts::existingFn","name":"existingFn","qualifiedName":"src-existing-ts::existingFn","isAsync":false,"isPublic":true,"paramCount":0,"lineStart":1,"lineEnd":3,"parameters":"","source":"function existingFn() {}","docstring":"","decorators":"","language":"typescript"}}"#;
    target_db
        .load_jsonl(existing_jsonl)
        .await
        .expect("load existing");

    // @step When I import the bundle with merge mode
    target_db
        .import_bundle(&bundle_path, AST_CODE_SCHEMA, "merge")
        .await
        .expect("import merge");

    // @step Then the graph contains both the existing and imported functions
    let stats = target_db.stats().expect("stats");
    let fn_count = stats
        .get("nodes")
        .and_then(|n| n.get("Function"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    // Should have both existingFn + extraFn = 2 functions
    assert!(
        fn_count >= 2,
        "expected at least 2 functions after merge, got {fn_count}"
    );
    let file_count = stats
        .get("nodes")
        .and_then(|n| n.get("File"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        file_count >= 2,
        "expected at least 2 files after merge, got {file_count}"
    );
}

// ============================================================================
// Scenario: Import rejects bundle with incompatible schema
// ============================================================================
#[tokio::test]
async fn test_import_rejects_incompatible_schema() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_export_db(temp_dir.path()).await;

    // @step Given an astbundle file created with a different schema version
    let bundle_path = temp_dir.path().join("bad-schema.astbundle");
    // Export with current schema
    db.export_bundle(&bundle_path, AST_CODE_SCHEMA)
        .await
        .expect("export");

    // Tamper with the schema inside the bundle to simulate a different version
    let tampered_path = temp_dir.path().join("tampered.astbundle");
    tamper_bundle_schema(
        &bundle_path,
        &tampered_path,
        "// MODIFIED SCHEMA\nnode Fake { slug: String @key }",
    );

    // @step When I attempt to import the bundle
    let import_db_path = temp_dir.path().join("reject-target.nano");
    let import_db = GraphDatabase::init(&import_db_path, AST_CODE_SCHEMA)
        .await
        .expect("init");
    let result = import_db
        .import_bundle(&tampered_path, AST_CODE_SCHEMA, "overwrite")
        .await;

    // @step Then the import fails with a schema mismatch error
    assert!(result.is_err(), "import should fail with schema mismatch");
    let err = result.unwrap_err();
    assert!(
        err.to_lowercase().contains("schema"),
        "error should mention schema: {err}"
    );

    // @step And no data in the graph is modified
    let stats = import_db.stats().expect("stats");
    let total_nodes: u64 = stats
        .get("nodes")
        .and_then(|n| n.as_object())
        .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum())
        .unwrap_or(0);
    assert_eq!(
        total_nodes, 0,
        "graph should remain empty after failed import"
    );
}
