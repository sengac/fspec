// Feature: spec/features/structural-extractors-zero-cost-indexing.feature
// Feature: spec/features/graphsearch-query-implementations.feature
// Feature: spec/features/deepsearch-graph-integration.feature
//
// End-to-end integration test: Full graph pipeline roundtrip.
// Verifies that structural extractors produce entities that load into the real
// nanograph DB and are searchable via the dispatch functions — the exact same
// code paths used at runtime.
//
// This test is the "prove it's WIRED" test the review demanded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::{
    ensure_graph_db, graph_db_stats, is_graph_initialized, reset_graph_db,
};
use codelet_napi::graph::dispatch;
use codelet_napi::graph::entity_pipeline::{
    extract_and_queue_from_tool_call, flush_pending_entities, take_pending_entities,
};
use codelet_napi::graph::extractors::{extract_from_file_operation, extract_from_fspec_command};
use codelet_napi::graph::merge::entities_to_jsonl;
use std::sync::Mutex;

// Global mutex — tests share the GRAPH_DB singleton
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    reset_graph_db();
    let _ = take_pending_entities();
    (guard, temp_dir)
}

const GRAPH_QUERIES: &str = include_str!("../schemas/graph-queries.gq");

// ============================================================================
// Scenario: Full pipeline — extract → queue → flush → verify Turn, CodeEntity, Modifies
// ============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn test_full_pipeline_extract_queue_flush_verify() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is initialized
    ensure_graph_db().await.expect("Graph init should succeed");
    assert!(is_graph_initialized());

    // @step When multiple Write tool calls are processed through the entity pipeline
    let files = [
        ("src/auth/login.rs", 0u32),
        ("src/auth/register.rs", 1),
        ("src/models/user.rs", 2),
    ];
    for (path, turn) in &files {
        let tool_args = serde_json::json!({
            "file_path": path,
            "content": "fn placeholder() {}"
        });
        extract_and_queue_from_tool_call("Write", &tool_args, "e2e-session", *turn);
    }

    // @step And the pending entities are flushed to the graph
    flush_pending_entities();

    // @step Then the queue is empty
    let remaining = take_pending_entities();
    assert!(remaining.is_empty(), "Queue should be empty after flush");

    // @step And the graph contains Turn, CodeEntity, and Modifies data
    let stats_json = graph_db_stats().await.expect("Stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();

    let turn_count = stats["nodes"]["Turn"].as_i64().unwrap_or(0);
    assert_eq!(turn_count, 3, "Should have exactly 3 Turn nodes, got {turn_count}");

    let ce_count = stats["nodes"]["CodeEntity"].as_i64().unwrap_or(0);
    assert_eq!(ce_count, 3, "Should have exactly 3 CodeEntity nodes, got {ce_count}");

    let mod_count = stats["edges"]["Modifies"].as_i64().unwrap_or(0);
    assert_eq!(mod_count, 3, "Should have exactly 3 Modifies edges, got {mod_count}");
}

// ============================================================================
// Scenario: Direct extractor → JSONL → load → query — CodeEntity searchable
// ============================================================================
#[tokio::test]
async fn test_extractor_to_query_code_entity_searchable() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given structural extractor output is loaded into the graph
    let entities = extract_from_file_operation("Write", "src/config/settings.ts", "sess-1", 5);
    let jsonl = entities_to_jsonl(&entities);
    codelet_napi::graph::graph_db_load_jsonl(&jsonl).await.expect("load should succeed");

    // @step Then the Turn node is present with correct session and turn index
    // (No query for Turn nodes by slug in graph-queries.gq, verify via stats)
    let stats_json = graph_db_stats().await.expect("stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    assert!(stats["nodes"]["Turn"].as_i64().unwrap_or(0) >= 1, "Turn node should exist");
    assert!(stats["nodes"]["CodeEntity"].as_i64().unwrap_or(0) >= 1, "CodeEntity should exist");
    assert!(stats["edges"]["Modifies"].as_i64().unwrap_or(0) >= 1, "Modifies edge should exist");
}

// ============================================================================
// Scenario: WorkUnit extractor → load → searchable
// ============================================================================
#[tokio::test]
async fn test_work_unit_extractor_to_query() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given an fspec create-story produces a WorkUnit entity
    let entities = extract_from_fspec_command("create-story", "AUTH-001", "Implement Login", "sess-1");
    let jsonl = entities_to_jsonl(&entities);
    codelet_napi::graph::graph_db_load_jsonl(&jsonl).await.expect("load should succeed");

    // @step Then the WorkUnit node is stored in the graph
    let stats_json = graph_db_stats().await.expect("stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    assert!(stats["nodes"]["WorkUnit"].as_i64().unwrap_or(0) >= 1, "WorkUnit should exist");
}

// ============================================================================
// Scenario: Concepts loaded → dispatch_search finds them → dispatch_neighbors works
// ============================================================================
#[tokio::test]
async fn test_dispatch_search_and_neighbors_on_loaded_data() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given concept nodes and RelatesTo edges are loaded
    let jsonl = r#"{"type":"Concept","data":{"slug":"vitest","name":"Vitest","category":"tool","summary":"Fast test runner","mentionCount":10,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"nanograph","name":"Nanograph","category":"library","summary":"Embedded graph DB","mentionCount":8,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"edge":"RelatesTo","from":"vitest","to":"nanograph","data":{"relationType":"uses","strength":0.7,"coOccurrenceCount":4,"firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;
    codelet_napi::graph::graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When dispatch_search is called for "Vitest"
    let search_result = dispatch::dispatch_search("Vitest", None, None).await;
    let parsed: serde_json::Value = serde_json::from_str(&search_result).unwrap();
    assert_eq!(parsed["action"], "search");
    let search_count = parsed["count"].as_i64().unwrap_or(0);
    assert!(search_count >= 1, "dispatch_search should find 'Vitest'");

    // @step And dispatch_neighbors is called for "vitest"
    let neighbors_result = dispatch::dispatch_neighbors("vitest", Some(1), None).await;
    let parsed: serde_json::Value = serde_json::from_str(&neighbors_result).unwrap();
    assert_eq!(parsed["action"], "neighbors");
    let neighbor_count = parsed["count"].as_i64().unwrap_or(0);
    assert!(neighbor_count >= 1, "dispatch_neighbors should find 'nanograph' as neighbor");

    // @step Then the neighbor is the expected concept
    let results = parsed["results"].as_array().unwrap();
    let slugs: Vec<&str> = results.iter()
        .filter_map(|r| r.get("slug").and_then(|v| v.as_str()))
        .collect();
    assert!(slugs.contains(&"nanograph"), "Neighbor should include 'nanograph'");
}

// ============================================================================
// Scenario: LLM extraction → load → dispatch_related returns co-occurring concepts
// ============================================================================
#[tokio::test]
async fn test_llm_extraction_to_dispatch_related() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given LLM-extracted entities are loaded into the graph
    let llm_response = r#"{
        "concepts": [
            {"slug": "acdd", "name": "Acceptance Criteria Driven Development", "category": "process", "summary": "TDD with Gherkin", "confidence": "high"},
            {"slug": "gherkin", "name": "Gherkin", "category": "tool", "summary": "BDD spec language", "confidence": "high"}
        ],
        "decisions": [],
        "relations": [
            {"from": "acdd", "to": "gherkin", "type": "uses", "strength": 0.9}
        ]
    }"#;
    let entities = codelet_napi::graph::llm_extraction::parse_and_validate_response(
        llm_response, "test-session", 0,
    ).expect("parse should succeed");
    let jsonl = entities_to_jsonl(&entities);
    codelet_napi::graph::graph_db_load_jsonl(&jsonl).await.expect("load should succeed");

    // @step When dispatch_related is called for "acdd"
    let result = dispatch::dispatch_related("acdd", None, None).await;
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["action"], "related");
    let count = parsed["count"].as_i64().unwrap_or(0);
    assert!(count >= 1, "dispatch_related should find 'gherkin' as related to 'acdd'");
}

// ============================================================================
// Scenario: Graph context injection into DeepSearch prompt with real DB data
// ============================================================================
#[tokio::test]
async fn test_deepsearch_context_injection_with_searchable_data() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given concepts exist in the graph
    let jsonl = r#"{"type":"Concept","data":{"slug":"fspec-cli","name":"fspec CLI","category":"tool","summary":"ACDD management tool","mentionCount":20,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;
    codelet_napi::graph::graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When the DeepSearch system prompt builder queries for related concepts
    let params = serde_json::json!({ "query": "fspec" });
    let results = codelet_napi::graph::graph_db_query(
        GRAPH_QUERIES, "search_concepts", Some(&params),
    ).await.expect("query should succeed");

    // @step Then concepts are found
    if let serde_json::Value::Array(concepts) = results {
        assert!(!concepts.is_empty(), "Should find fspec concept");

        // @step And build_graph_context returns a context section
        let context = codelet_napi::graph::deepsearch_integration::build_graph_context(&concepts);
        assert!(context.is_some(), "Context should be built from found concepts");
        let ctx = context.unwrap();
        assert!(ctx.contains("fspec CLI"), "Context should contain concept name");
        assert!(ctx.contains("ACDD management tool"), "Context should contain summary");
    } else {
        panic!("Expected array result from search_concepts");
    }
}

// ============================================================================
// Scenario: Index action flushes pending entities and returns status
// ============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn test_dispatch_index_flushes_pending_and_returns_status() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given pending entities exist in the entity pipeline queue
    ensure_graph_db().await.expect("Graph init should succeed");
    let tool_args = serde_json::json!({
        "file_path": "src/index_test.rs",
        "content": "fn test() {}"
    });
    extract_and_queue_from_tool_call("Write", &tool_args, "idx-session", 0);

    // @step When the index action is invoked
    let result = dispatch::dispatch_index(None).await;
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["action"], "index");

    // @step Then pending entities are flushed to the graph database
    let remaining = take_pending_entities();
    assert!(remaining.is_empty(), "Queue should be empty after index action");

    // @step Then the result indicates the indexing status as JSON
    let status = parsed["status"].as_str().unwrap_or("unknown");
    assert_eq!(status, "indexed", "Status should be 'indexed', got '{status}'");
}

// ============================================================================
// Scenario: History action returns turn provenance for a concept
// ============================================================================
#[tokio::test]
async fn test_dispatch_history_returns_turn_provenance() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given a knowledge graph with Turn nodes linked to concepts via Mentions edges
    let jsonl = r#"{"type":"Concept","data":{"slug":"redis","name":"Redis","category":"technology","summary":"In-memory cache","mentionCount":5,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Turn","data":{"slug":"sess-1:3","sessionSlug":"sess-1","turnIndex":3,"role":"assistant","timestamp":"2026-03-19T10:00:00Z","preview":"Let's use Redis for caching"}}
{"edge":"Mentions","from":"sess-1:3","to":"redis","data":{"confidence":"high","extractedAt":"2026-03-19T10:00:00Z"}}"#;
    codelet_napi::graph::graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When the history action is invoked for concept 'redis'
    let result = dispatch::dispatch_history("redis", None).await;
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["action"], "history");

    // @step Then Turn nodes that mention the concept are returned with session and turn index
    let count = parsed["count"].as_i64().unwrap_or(0);
    assert!(count >= 1, "dispatch_history should find at least 1 turn mentioning redis, got {count}");

    let results = parsed["results"].as_array().unwrap();
    let first = &results[0];
    assert_eq!(
        first.get("slug").and_then(|v| v.as_str()),
        Some("sess-1:3"),
        "Should return the Turn that mentions redis"
    );
}
