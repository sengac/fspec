// Feature: spec/features/portable-graph-bundles.feature
//
// Portable Graph Bundles — Export/Import
// Tests that the AST graph can be exported to a portable .astbundle ZIP archive
// and imported back, preserving all nodes, edges, and properties.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::{entities_to_jsonl, jsonl_to_entities, GraphEntity};
use serde_json::{Map, Value};
use std::io::Read;

/// The AST code schema.
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create a populated graph for export testing.
async fn setup_export_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-export.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-main-ts","path":"src/main.ts","language":"typescript","lineCount":150,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-py","path":"src/utils.py","language":"python","lineCount":80,"isTest":false}}
{"type":"Function","data":{"slug":"src-main-ts::handleRequest","name":"handleRequest","qualifiedName":"src-main-ts::handleRequest","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":10,"lineEnd":30,"cyclomaticComplexity":4,"parameters":"req, res","source":"async function handleRequest(req, res) { ... }","docstring":"Handle incoming request","decorators":"@Route","language":"typescript"}}
{"type":"Function","data":{"slug":"src-utils-py::parse","name":"parse","qualifiedName":"src-utils-py::parse","isAsync":false,"isPublic":true,"paramCount":1,"lineStart":5,"lineEnd":15,"cyclomaticComplexity":2,"parameters":"data","source":"def parse(data): ...","docstring":"Parse data","decorators":"@staticmethod","language":"python"}}
{"type":"Type","data":{"slug":"src-main-ts::Config","name":"Config","typeKind":"interface","isPublic":true,"lineStart":1,"lineEnd":5,"source":"interface Config { port: number; }","docstring":"App config","decorators":"","language":"typescript"}}
{"type":"Dependency","data":{"slug":"dep-express","name":"express","version":"4.18.2","isDev":false,"source":"npm"}}
{"edge":"Contains","from":"src-main-ts","to":"src-main-ts::handleRequest","data":{"lineStart":10}}
{"edge":"Contains","from":"src-utils-py","to":"src-utils-py::parse","data":{"lineStart":5}}
{"edge":"ContainsType","from":"src-main-ts","to":"src-main-ts::Config","data":{"lineStart":1}}
{"edge":"Imports","from":"src-main-ts","to":"src-utils-py","data":{"importPath":"./utils"}}
{"edge":"DependsOn","from":"src-main-ts","to":"dep-express","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

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

/// Helper: create a tampered bundle with a different schema.pg
fn tamper_bundle_schema(
    source_path: &std::path::Path,
    target_path: &std::path::Path,
    fake_schema: &str,
) {
    let source_file = std::fs::File::open(source_path).expect("open source");
    let mut source_archive = zip::ZipArchive::new(source_file).expect("read source ZIP");

    let target_file = std::fs::File::create(target_path).expect("create target");
    let mut writer = zip::ZipWriter::new(target_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for i in 0..source_archive.len() {
        let mut entry = source_archive.by_index(i).expect("read entry");
        let name = entry.name().to_string();

        if name == "schema.pg" {
            writer.start_file(&name, options).expect("start schema");
            std::io::Write::write_all(&mut writer, fake_schema.as_bytes()).expect("write schema");
        } else {
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).expect("read entry");
            writer.start_file(&name, options).expect("start file");
            std::io::Write::write_all(&mut writer, &contents).expect("write file");
        }
    }
    writer.finish().expect("finish ZIP");
}

// ============================================================================
// Scenario: Export and import round-trip preserves all data
// ============================================================================
#[tokio::test]
async fn test_export_import_roundtrip_preserves_data() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_export_db(temp_dir.path()).await;

    // @step Given a graph indexed with multiple node types and edge types
    let original_stats = db.stats().expect("stats");

    // @step When I export the graph to a bundle
    let bundle_path = temp_dir.path().join("roundtrip.astbundle");
    db.export_bundle(&bundle_path, AST_CODE_SCHEMA)
        .await
        .expect("export");

    // @step And I reset the graph
    let fresh_db_path = temp_dir.path().join("roundtrip-fresh.nano");
    let fresh_db = GraphDatabase::init(&fresh_db_path, AST_CODE_SCHEMA)
        .await
        .expect("init fresh DB");

    // @step And I import the bundle
    fresh_db
        .import_bundle(&bundle_path, AST_CODE_SCHEMA, "overwrite")
        .await
        .expect("import bundle");

    // @step Then ast_stats shows the same node and edge counts as before export
    let roundtrip_stats = fresh_db.stats().expect("roundtrip stats");
    assert_eq!(
        original_stats.get("nodes"),
        roundtrip_stats.get("nodes"),
        "node counts should match after round-trip"
    );
    assert_eq!(
        original_stats.get("edges"),
        roundtrip_stats.get("edges"),
        "edge counts should match after round-trip"
    );
}

// ============================================================================
// Scenario: JSONL round-trip serializes and deserializes all entity types
// ============================================================================
#[tokio::test]
async fn test_jsonl_roundtrip_serialization() {
    // @step Given a list of GraphEntity nodes and edges with various property types
    let mut fn_props = Map::new();
    fn_props.insert("slug".into(), Value::String("file::myFunc".into()));
    fn_props.insert("name".into(), Value::String("myFunc".into()));
    fn_props.insert("isAsync".into(), Value::Bool(true));
    fn_props.insert("paramCount".into(), Value::Number(3.into()));
    fn_props.insert("language".into(), Value::String("typescript".into()));

    let mut edge_props = Map::new();
    edge_props.insert("lineStart".into(), Value::Number(42.into()));

    let entities = vec![
        GraphEntity::Node {
            node_type: "Function".into(),
            slug: "file::myFunc".into(),
            properties: fn_props.clone(),
        },
        GraphEntity::Edge {
            edge_type: "Contains".into(),
            from_slug: "file-slug".into(),
            to_slug: "file::myFunc".into(),
            properties: edge_props.clone(),
        },
    ];

    // @step When I serialize them with entities_to_jsonl
    let jsonl = entities_to_jsonl(&entities);

    // @step And I deserialize the result with jsonl_to_entities
    let deserialized = jsonl_to_entities(&jsonl).expect("deserialize JSONL");

    // @step Then the deserialized entities match the originals in type, slugs, and properties
    assert_eq!(deserialized.len(), 2, "should have 2 entities");

    // Check node
    match &deserialized[0] {
        GraphEntity::Node {
            node_type,
            slug,
            properties,
        } => {
            assert_eq!(node_type, "Function");
            assert_eq!(slug, "file::myFunc");
            assert_eq!(
                properties.get("name").and_then(|v| v.as_str()),
                Some("myFunc")
            );
            assert_eq!(
                properties.get("isAsync").and_then(|v| v.as_bool()),
                Some(true)
            );
            assert_eq!(
                properties.get("paramCount").and_then(|v| v.as_i64()),
                Some(3)
            );
        }
        other => panic!("expected Node, got {:?}", other),
    }

    // Check edge
    match &deserialized[1] {
        GraphEntity::Edge {
            edge_type,
            from_slug,
            to_slug,
            properties,
        } => {
            assert_eq!(edge_type, "Contains");
            assert_eq!(from_slug, "file-slug");
            assert_eq!(to_slug, "file::myFunc");
            assert_eq!(
                properties.get("lineStart").and_then(|v| v.as_i64()),
                Some(42)
            );
        }
        other => panic!("expected Edge, got {:?}", other),
    }
}
