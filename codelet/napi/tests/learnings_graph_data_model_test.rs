#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect, clippy::await_holding_lock)]
// Feature: spec/features/learnings-graph-data-model.feature
//
// Learnings Graph Data Model & Schema
// Tests for the Learnings graph schema, JSONL loading, edge traversal,
// and registry integration.
//
// Each test uses an isolated temp directory to avoid polluting real data.


use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::registry;
use serde_json::Value;
use std::sync::Mutex;

/// The Learnings schema, same as what's bundled in the binary.
const LEARNINGS_SCHEMA: &str = include_str!("../../graph/schemas/learnings.pg");

/// Inline query source for traversing Learnings graph.
const LEARNINGS_QUERIES: &str = r#"
query all_learnings() {
    match { $l: Learning }
    return { $l.slug, $l.title, $l.category, $l.confidence, $l.content, $l.mentionCount }
}

query all_explorations() {
    match { $e: Exploration }
    return { $e.slug, $e.title, $e.strategy, $e.outcome }
}

query exploration_discoveries($exp_slug: String) {
    match {
        $e: Exploration { slug: $exp_slug }
        $e discovered $l
    }
    return { $l.slug, $l.title, $l.category }
}

query learning_related($learn_slug: String) {
    match {
        $src: Learning { slug: $learn_slug }
        $src relatesTo $dst
    }
    return { $dst.slug, $dst.title, $dst.category }
}

query learning_superseded_by($learn_slug: String) {
    match {
        $old: Learning { slug: $learn_slug }
        $new supersedes $old
    }
    return { $new.slug, $new.title }
}
"#;

// Global mutex for tests that use the global registry (scenario 4).
lazy_static::lazy_static! {
    static ref REGISTRY_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

// ============================================================================
// Scenario: Initialize Learnings graph database with schema
// ============================================================================
#[tokio::test]
async fn test_initialize_learnings_graph_database_with_schema() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("learnings.nano");

    // @step Given the global data directory exists
    assert!(temp_dir.path().exists());

    // @step And no Learnings graph database has been initialized
    assert!(!db_path.exists(), "Database should not exist yet");

    // @step When the Learnings graph database is initialized
    let db = GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("Learnings graph init should succeed");

    // @step Then the database should be created at "~/.fspec/graph/learnings.nano/"
    assert!(db_path.exists(), "Database directory should exist after init");
    assert!(
        db_path.join("schema.ir.json").exists(),
        "schema.ir.json should exist"
    );

    // @step And the schema catalog should contain node types "Learning, Exploration, Convention, Decision, CodePattern"
    let node_names = db.node_type_names();
    for expected in &[
        "Learning",
        "Exploration",
        "Convention",
        "Decision",
        "CodePattern",
    ] {
        assert!(
            node_names.contains(&expected.to_string()),
            "Schema should contain node type '{}', got: {:?}",
            expected,
            node_names
        );
    }

    // @step And the schema catalog should contain edge types "Discovered, Eliminates, Supersedes, RelatesTo, InformedBy, Applies, Contradicts"
    let edge_names = db.edge_type_names();
    for expected in &[
        "Discovered",
        "Eliminates",
        "Supersedes",
        "RelatesTo",
        "InformedBy",
        "Applies",
        "Contradicts",
    ] {
        assert!(
            edge_names.contains(&expected.to_string()),
            "Schema should contain edge type '{}', got: {:?}",
            expected,
            edge_names
        );
    }

    // @step And all node types should have a "slug" key property
    for node_type in &[
        "Learning",
        "Exploration",
        "Convention",
        "Decision",
        "CodePattern",
    ] {
        assert!(
            db.node_has_property(node_type, "slug"),
            "Node type '{}' should have 'slug' property",
            node_type
        );
    }
}

// ============================================================================
// Scenario: Load batch of Learning and Exploration nodes via JSONL
// ============================================================================
#[tokio::test]
async fn test_load_batch_learning_and_exploration_nodes_via_jsonl() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("learnings.nano");

    // @step Given the Learnings graph database is initialized
    let db = GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("Init should succeed");
    let db = db.with_query_source(LEARNINGS_QUERIES);

    // @step When I load a batch of JSONL containing Learning and Exploration nodes
    let jsonl = [
        r#"{"type":"Learning","data":{"slug":"use-bool-not-boolean","title":"Use Bool not Boolean in nanograph schemas","content":"Nanograph PG parser requires Bool type, not Boolean","category":"convention","confidence":"high","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":3}}"#,
        r#"{"type":"Learning","data":{"slug":"batch-jsonl-loading","title":"Always use batch JSONL loading","content":"Loading entities one at a time causes Lance version amplification","category":"pattern","confidence":"high","firstSeen":"2026-03-20T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":5}}"#,
        r#"{"type":"Learning","data":{"slug":"singleton-registry-pattern","title":"Use registry for multiple graph instances","content":"A HashMap-based registry with lazy_static manages named graph singletons","category":"pattern","confidence":"medium","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":1}}"#,
        r#"{"type":"Exploration","data":{"slug":"exp-full-indexing","title":"Full conversation indexing attempt","strategy":"Index ALL session turns into graph database","outcome":"failure","failureConstraint":"7.6GB disk usage after 727 turns — unsustainable","survivingStructure":"Session scanner and entity pipeline are reusable","createdAt":"2026-03-20T00:00:00"}}"#,
        r#"{"type":"Exploration","data":{"slug":"exp-dual-graph","title":"Dual-graph architecture exploration","strategy":"Separate AST graph (code structure) from Learnings graph (knowledge)","outcome":"success","createdAt":"2026-03-22T00:00:00"}}"#,
    ]
    .join("\n");

    db.load_jsonl(&jsonl)
        .await
        .expect("Batch JSONL load should succeed");

    // @step Then querying for Learning nodes should return the loaded learnings with correct categories
    let learnings = db
        .query("all_learnings", None)
        .await
        .expect("all_learnings query should succeed");
    let learnings_arr = learnings.as_array().expect("Result should be an array");
    assert_eq!(learnings_arr.len(), 3, "Should have loaded 3 Learning nodes");

    let categories: Vec<&str> = learnings_arr
        .iter()
        .filter_map(|l| l.get("category").and_then(Value::as_str))
        .collect();
    assert!(
        categories.contains(&"convention"),
        "Should contain a convention learning"
    );
    assert!(
        categories.contains(&"pattern"),
        "Should contain pattern learnings"
    );

    // @step And querying for Exploration nodes should return the loaded explorations with correct properties
    let explorations = db
        .query("all_explorations", None)
        .await
        .expect("all_explorations query should succeed");
    let exps_arr = explorations.as_array().expect("Result should be an array");
    assert_eq!(
        exps_arr.len(),
        2,
        "Should have loaded 2 Exploration nodes"
    );

    let outcomes: Vec<&str> = exps_arr
        .iter()
        .filter_map(|e| e.get("outcome").and_then(Value::as_str))
        .collect();
    assert!(outcomes.contains(&"failure"), "Should contain failed exploration");
    assert!(outcomes.contains(&"success"), "Should contain successful exploration");

    // @step And no Lance version amplification should occur from the batch load
    let stats = db.stats().expect("Stats should succeed");
    let learning_count = stats
        .pointer("/nodes/Learning")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(learning_count, 3, "Learning node count should be 3");
    let exp_count = stats
        .pointer("/nodes/Exploration")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    assert_eq!(exp_count, 2, "Exploration node count should be 2");
}

// ============================================================================
// Scenario: Load relationship edges and traverse connections
// ============================================================================
#[tokio::test]
async fn test_load_relationship_edges_and_traverse_connections() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("learnings.nano");

    // @step Given the Learnings graph database is initialized
    let db = GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("Init should succeed");
    let db = db.with_query_source(LEARNINGS_QUERIES);

    // @step And Learning and Exploration nodes have been loaded
    let nodes_jsonl = [
        r#"{"type":"Learning","data":{"slug":"use-bool","title":"Use Bool type","content":"Nanograph requires Bool","category":"convention","confidence":"high","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":1}}"#,
        r#"{"type":"Learning","data":{"slug":"batch-load","title":"Batch JSONL loading","content":"Avoid per-entity loading","category":"pattern","confidence":"high","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":1}}"#,
        r#"{"type":"Learning","data":{"slug":"old-pattern","title":"Old singleton pattern","content":"The old mod.rs pattern duplicated registry code","category":"anti_pattern","confidence":"medium","firstSeen":"2026-03-20T00:00:00","lastSeen":"2026-03-20T00:00:00","mentionCount":1}}"#,
        r#"{"type":"Learning","data":{"slug":"registry-pattern","title":"Registry pattern","content":"Use registry.rs for named graph instances","category":"pattern","confidence":"high","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00","mentionCount":1}}"#,
        r#"{"type":"Exploration","data":{"slug":"exp-refactor","title":"Graph module refactoring","strategy":"Extract GraphDatabase abstraction from mod.rs singleton","outcome":"success","createdAt":"2026-03-22T00:00:00"}}"#,
    ]
    .join("\n");
    db.load_jsonl(&nodes_jsonl)
        .await
        .expect("Node load should succeed");

    // @step When I load Discovered edges linking explorations to learnings
    let discovered_jsonl = [
        r#"{"edge":"Discovered","from":"exp-refactor","to":"registry-pattern","data":{"extractedAt":"2026-03-22T00:00:00"}}"#,
        r#"{"edge":"Discovered","from":"exp-refactor","to":"batch-load","data":{"extractedAt":"2026-03-22T00:00:00"}}"#,
    ]
    .join("\n");
    db.load_jsonl(&discovered_jsonl)
        .await
        .expect("Discovered edge load should succeed");

    // @step And I load Supersedes edges between learnings
    let supersedes_jsonl =
        r#"{"edge":"Supersedes","from":"registry-pattern","to":"old-pattern","data":{"supersededAt":"2026-03-22T00:00:00","reason":"Registry is cleaner than monolithic singleton"}}"#;
    db.load_jsonl(supersedes_jsonl)
        .await
        .expect("Supersedes edge load should succeed");

    // @step And I load RelatesTo edges between learnings
    let relates_jsonl =
        r#"{"edge":"RelatesTo","from":"batch-load","to":"use-bool","data":{"strength":0.7,"relationType":"uses","firstSeen":"2026-03-22T00:00:00","lastSeen":"2026-03-22T00:00:00"}}"#;
    db.load_jsonl(relates_jsonl)
        .await
        .expect("RelatesTo edge load should succeed");

    // @step Then traversing neighbors of a learning node should return related learnings
    let related = db
        .query(
            "learning_related",
            Some(&serde_json::json!({"learn_slug": "batch-load"})),
        )
        .await
        .expect("learning_related query should succeed");
    let related_arr = related.as_array().expect("Related should be an array");
    assert_eq!(related_arr.len(), 1, "batch-load should relate to 1 learning");
    assert_eq!(
        related_arr[0].get("slug").and_then(Value::as_str),
        Some("use-bool"),
        "Related learning should be use-bool"
    );

    // @step And traversing neighbors of an exploration should return its discovered learnings
    let discoveries = db
        .query(
            "exploration_discoveries",
            Some(&serde_json::json!({"exp_slug": "exp-refactor"})),
        )
        .await
        .expect("exploration_discoveries query should succeed");
    let disc_arr = discoveries.as_array().expect("Discoveries should be an array");
    assert_eq!(
        disc_arr.len(),
        2,
        "exp-refactor should have discovered 2 learnings"
    );
    let disc_slugs: Vec<&str> = disc_arr
        .iter()
        .filter_map(|d| d.get("slug").and_then(Value::as_str))
        .collect();
    assert!(
        disc_slugs.contains(&"registry-pattern"),
        "Should have discovered registry-pattern"
    );
    assert!(
        disc_slugs.contains(&"batch-load"),
        "Should have discovered batch-load"
    );
}

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

    // @step When I request the Learnings graph by name "learnings"
    let learnings_db = registry::get_graph(registry::LEARNINGS_GRAPH)
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
    registry::reset_all_graphs();
}
