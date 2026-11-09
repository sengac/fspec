// Feature: spec/features/portable-graph-bundles.feature
use super::*;

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
