// Feature: spec/features/structural-extractors-zero-cost-indexing.feature
// Feature: spec/features/deepsearch-graph-integration.feature
//
// Integration tests for the entity pipeline (KGRAPH-004) and DeepSearch graph
// integration (KGRAPH-009). Tests the ACTUAL wiring, not mocks.
//
// CRIT-002: entity_pipeline.rs previously had zero test coverage
// CRIT-003: "Extractors silently skip" was a false coverage scenario
// CRIT-005: DeepSearch integration was tested only at mock level

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::{
    ensure_graph_db, graph_db_stats, is_graph_initialized, reset_graph_db,
};
use codelet_napi::graph::entity_pipeline::{
    extract_and_queue_from_tool_call, flush_pending_entities, take_pending_entities,
};
use std::sync::Mutex;

// Global mutex — these tests share global singletons so must run sequentially
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    reset_graph_db();
    // Drain any leftover entities from previous tests (global queue)
    let _ = take_pending_entities();
    (guard, temp_dir)
}

// ============================================================================
// Scenario: Extractors silently skip when graph is unavailable (KGRAPH-004)
// This tests the ACTUAL guard in entity_pipeline, not the pure extractors
// ============================================================================
#[tokio::test]
async fn test_extract_and_queue_silently_skips_when_graph_unavailable() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is NOT initialized
    assert!(
        !is_graph_initialized(),
        "Graph should not be initialized after reset"
    );

    // @step When a Write tool call is processed through entity_pipeline
    let tool_args = serde_json::json!({
        "file_path": "src/auth/login.rs",
        "content": "fn login() {}"
    });
    extract_and_queue_from_tool_call("Write", &tool_args, "test-session", 0);

    // @step Then no entities are queued and no error is raised
    let pending = take_pending_entities();
    assert!(
        pending.is_empty(),
        "No entities should be queued when graph is unavailable, got {}",
        pending.len()
    );
}

// ============================================================================
// Scenario: Entity pipeline queues entities when graph IS initialized
// ============================================================================
#[tokio::test]
async fn test_extract_and_queue_produces_entities_when_graph_available() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is initialized
    ensure_graph_db().await.expect("Graph init should succeed");
    assert!(is_graph_initialized(), "Graph should be initialized");

    // @step When a Write tool call is processed
    let tool_args = serde_json::json!({
        "file_path": "src/auth/login.rs",
        "content": "fn login() {}"
    });
    extract_and_queue_from_tool_call("Write", &tool_args, "test-session", 1);

    // @step Then entities are queued in the pending queue
    let pending = take_pending_entities();
    assert!(
        !pending.is_empty(),
        "Entities should be queued when graph is initialized"
    );
}

// ============================================================================
// Scenario: Fspec tool call produces WorkUnit entities via pipeline
// ============================================================================
#[tokio::test]
async fn test_fspec_tool_call_queues_work_unit_entity() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step When an Fspec create-story tool call is processed
    let tool_args = serde_json::json!({
        "command": "create-story",
        "args": { "_": ["AUTH-001", "User Login"] }
    });
    extract_and_queue_from_tool_call("Fspec", &tool_args, "test-session", 2);

    // @step Then a WorkUnit entity is queued
    let pending = take_pending_entities();
    assert!(
        !pending.is_empty(),
        "WorkUnit entity should be queued from Fspec create-story"
    );
}

// ============================================================================
// Scenario: Flush pending entities writes to graph database
// ============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn test_flush_pending_entities_writes_to_db() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // Queue entities through the pipeline
    for i in 0..3u32 {
        let tool_args = serde_json::json!({
            "file_path": format!("src/mod_{}.rs", i),
            "content": "fn test() {}"
        });
        extract_and_queue_from_tool_call("Write", &tool_args, "flush-session", i);
    }

    // @step When flush_pending_entities is called
    flush_pending_entities();

    // @step Then the pending queue is empty after flush
    let remaining = take_pending_entities();
    assert!(
        remaining.is_empty(),
        "Pending queue should be empty after flush"
    );

    // @step And entities are loaded into the graph (Turn nodes + CodeEntity nodes + Modifies edges)
    let stats_json = graph_db_stats().await.expect("Stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();

    let turn_count = stats["nodes"]["Turn"].as_i64().unwrap_or(0);
    assert!(
        turn_count >= 3,
        "Should have at least 3 Turn nodes (one per tool call), got {turn_count}"
    );

    let code_entity_count = stats["nodes"]["CodeEntity"].as_i64().unwrap_or(0);
    assert!(
        code_entity_count >= 3,
        "Should have at least 3 CodeEntity nodes (one per file), got {code_entity_count}"
    );

    let modifies_count = stats["edges"]["Modifies"].as_i64().unwrap_or(0);
    assert!(
        modifies_count >= 3,
        "Should have at least 3 Modifies edges (Turn → CodeEntity), got {modifies_count}"
    );
}

// ============================================================================
// Scenario: flush_pending_entities silently returns when graph unavailable
// ============================================================================
#[tokio::test]
async fn test_flush_silently_returns_when_graph_unavailable() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is NOT initialized
    assert!(!is_graph_initialized());

    // @step When flush_pending_entities is called
    flush_pending_entities(); // Should not panic or error

    // @step Then no error is raised (function returns cleanly)
    // If we got here without panic, the test passes
}

// ============================================================================
// Scenario: Unrecognized tool names are silently ignored
// ============================================================================
#[tokio::test]
async fn test_unrecognized_tool_name_silently_ignored() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step When an unrecognized tool name is processed
    let tool_args = serde_json::json!({ "query": "test" });
    extract_and_queue_from_tool_call("Read", &tool_args, "test-session", 0);
    extract_and_queue_from_tool_call("Grep", &tool_args, "test-session", 0);
    extract_and_queue_from_tool_call("Bash", &tool_args, "test-session", 0);

    // @step Then no entities are queued
    let pending = take_pending_entities();
    assert!(
        pending.is_empty(),
        "Unrecognized tool names should not queue entities"
    );
}

// ============================================================================
// Scenario: DeepSearch system prompt includes graph context when data exists
// (KGRAPH-009 — tests the actual build_graph_context function)
// ============================================================================
#[tokio::test]
async fn test_deepsearch_graph_context_with_real_data() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step Given concepts exist in the graph database
    let jsonl = r#"{"type":"Concept","data":{"slug":"jwt-authentication","name":"JWT Authentication","category":"technology","summary":"Token-based auth","mentionCount":10,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"session-management","name":"Session Management","category":"pattern","summary":"Server sessions","mentionCount":5,"confidence":"medium","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;

    codelet_napi::graph::graph_db_load_jsonl(jsonl)
        .await
        .expect("Load should succeed");

    // @step When related concepts are queried for the DeepSearch prompt
    let query_source = include_str!("../schemas/graph-queries.gq");
    let params = serde_json::json!({ "query": "JWT" });
    let results = codelet_napi::graph::graph_db_query(query_source, "search_concepts", Some(&params))
        .await
        .expect("Search should succeed");

    if let serde_json::Value::Array(concepts) = results {
        // @step Then build_graph_context returns a context section
        let context =
            codelet_napi::graph::deepsearch_integration::build_graph_context(&concepts);
        assert!(
            context.is_some(),
            "build_graph_context should return Some when concepts exist"
        );
        let context_str = context.unwrap();

        // @step And the context includes concept names
        assert!(
            context_str.contains("JWT Authentication"),
            "Context should include concept name"
        );
        assert!(
            context_str.contains("Knowledge graph context"),
            "Context should have knowledge graph header"
        );
    } else {
        panic!("Search should return an array");
    }
}

// ============================================================================
// Scenario: DeepSearch works without graph database (backward compatible)
// ============================================================================
#[tokio::test]
async fn test_deepsearch_without_graph_returns_no_context() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is NOT initialized
    assert!(!is_graph_initialized());

    // @step When build_graph_context is called with empty concepts
    let context =
        codelet_napi::graph::deepsearch_integration::build_graph_context(&[]);

    // @step Then no context is returned
    assert!(
        context.is_none(),
        "build_graph_context should return None for empty concepts"
    );
}
