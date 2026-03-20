// Feature: spec/features/llm-concept-extraction.feature
// Feature: spec/features/graphsearch-tool-definition-handler-registration.feature
//
// Integration tests for LLM extraction → graph DB round-trip (KGRAPH-005)
// and full dispatch handler end-to-end (KGRAPH-003/007).
//
// CRIT-7: KGRAPH-005 previously had unit tests only — no real DB integration.
// These tests verify that LLM extraction output actually loads into nanograph
// and is queryable via the dispatch functions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::{
    ensure_graph_db, graph_db_load_jsonl, graph_db_query, graph_db_stats,
    reset_graph_db,
};
use codelet_napi::graph::llm_extraction::parse_and_validate_response;
use codelet_napi::graph::merge::entities_to_jsonl;
use codelet_napi::graph::dispatch;
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
    (guard, temp_dir)
}

// ============================================================================
// Scenario: Valid LLM response produces entities that load into graph DB
// (KGRAPH-005 integration — validates the full pipeline)
// ============================================================================
#[tokio::test]
async fn test_llm_extraction_entities_load_into_graph() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given a well-formed JSON response from the LLM containing concepts, decisions, and relations
    let llm_response = r#"{
        "concepts": [
            {"slug": "jwt-auth", "name": "JWT Authentication", "category": "technology", "summary": "Token-based stateless auth", "confidence": "high"},
            {"slug": "session-mgmt", "name": "Session Management", "category": "pattern", "summary": "Server-side sessions", "confidence": "medium"}
        ],
        "decisions": [
            {"slug": "use-jwt", "title": "Use JWT for authentication", "rationale": "Stateless and scalable", "domain": "architecture", "confidence": "high"}
        ],
        "relations": [
            {"from": "jwt-auth", "to": "session-mgmt", "type": "supersedes", "strength": 0.8}
        ]
    }"#;

    // @step When the response is parsed and validated
    let entities = parse_and_validate_response(llm_response, "test-session", 0)
        .expect("Parsing should succeed");

    // Expect: 2 concepts + 1 Turn node + 1 decision + 1 Decides edge + 1 RelatesTo edge = 6 entities
    assert!(
        entities.len() >= 5,
        "Should produce at least 5 entities (2 concepts + 1 Turn + 1 decision + 1 edge), got {}",
        entities.len()
    );

    // @step And the entities are loaded into the graph database
    // The Turn node is now emitted inline by validate_decision(), so no manual
    // Turn creation is needed — the Decides edge resolves via the co-emitted Turn.
    ensure_graph_db().await.expect("Graph init should succeed");

    let jsonl = entities_to_jsonl(&entities);
    graph_db_load_jsonl(&jsonl).await.expect("JSONL load should succeed");

    // @step Then the concepts are queryable via search
    let query_source = include_str!("../schemas/graph-queries.gq");
    let params = serde_json::json!({ "query": "JWT" });
    let results = graph_db_query(query_source, "search_concepts", Some(&params))
        .await
        .expect("Search should succeed");

    let arr = results.as_array().expect("Should be array");
    assert!(
        !arr.is_empty(),
        "Search for 'JWT' should find the loaded concept"
    );

    // @step And the stats show non-zero counts for Concept and Decision nodes
    let stats_json = graph_db_stats().await.expect("Stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let concept_count = stats["nodes"]["Concept"].as_i64().unwrap_or(0);
    let decision_count = stats["nodes"]["Decision"].as_i64().unwrap_or(0);
    assert!(concept_count >= 2, "Should have at least 2 Concept nodes, got {concept_count}");
    assert!(decision_count >= 1, "Should have at least 1 Decision node, got {decision_count}");
}

// ============================================================================
// Scenario: Malformed LLM responses don't corrupt the graph
// ============================================================================
#[tokio::test]
async fn test_malformed_llm_response_safe_to_load() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step Given a JSON response with one valid and one malformed concept
    let response = r#"{
        "concepts": [
            {"slug": "", "name": "Empty slug", "category": "technology"},
            {"slug": "valid-concept", "name": "Valid Concept", "category": "tool", "confidence": "high"}
        ],
        "decisions": [],
        "relations": []
    }"#;

    // @step When the response is parsed (malformed entries are filtered)
    let entities = parse_and_validate_response(response, "test-session", 0)
        .expect("Parsing should succeed even with malformed entries");
    assert_eq!(entities.len(), 1, "Only valid concept should survive validation");

    // @step And the valid entities are loaded into the graph
    let jsonl = entities_to_jsonl(&entities);
    graph_db_load_jsonl(&jsonl).await.expect("Load should succeed");

    // @step Then the graph contains only the valid concept
    let stats_json = graph_db_stats().await.expect("Stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let concept_count = stats["nodes"]["Concept"].as_i64().unwrap_or(0);
    assert_eq!(concept_count, 1, "Only valid concept should be in graph");
}

// ============================================================================
// Scenario: Full dispatch search end-to-end
// (Tests the actual dispatch functions used by the handler)
// ============================================================================
#[tokio::test]
async fn test_dispatch_search_end_to_end() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step Given concepts are loaded into the graph
    let jsonl = r#"{"type":"Concept","data":{"slug":"rust-lang","name":"Rust Programming Language","category":"technology","summary":"Systems language","mentionCount":15,"confidence":"high","firstSeen":"2026-01-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"typescript","name":"TypeScript","category":"technology","summary":"JS superset","mentionCount":20,"confidence":"high","firstSeen":"2026-01-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;
    graph_db_load_jsonl(jsonl).await.expect("Load should succeed");

    // @step When the dispatch_search function is called
    let result = dispatch::dispatch_search("Rust", None, None).await;

    // @step Then the result is valid JSON with matching concepts
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("dispatch_search should return valid JSON");
    assert_eq!(parsed["action"], "search");
    assert_eq!(parsed["query"], "Rust");
    let count = parsed["count"].as_i64().unwrap_or(0);
    assert!(count >= 1, "Should find at least one matching concept");
}

// ============================================================================
// Scenario: Full dispatch decisions end-to-end
// ============================================================================
#[tokio::test]
async fn test_dispatch_decisions_end_to_end() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // @step Given decisions are loaded into the graph
    let jsonl = r#"{"type":"Decision","data":{"slug":"use-nanograph","title":"Use Nanograph for Graph DB","rationale":"Embedded, typed","domain":"architecture","status":"active","confidence":"high","decidedAt":"2026-03-19T00:00:00Z","createdAt":"2026-03-19T00:00:00Z","session":"test"}}
{"type":"Decision","data":{"slug":"use-vitest","title":"Use Vitest for Testing","rationale":"Fast, ESM native","domain":"testing","status":"active","confidence":"high","decidedAt":"2026-03-18T00:00:00Z","createdAt":"2026-03-18T00:00:00Z","session":"test"}}"#;
    graph_db_load_jsonl(jsonl).await.expect("Load should succeed");

    // @step When dispatch_decisions is called with domain filter "architecture"
    let result = dispatch::dispatch_decisions(Some("architecture"), None, None).await;

    // @step Then only architecture decisions are returned
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("dispatch_decisions should return valid JSON");
    assert_eq!(parsed["action"], "decisions");
    let count = parsed["count"].as_i64().unwrap_or(0);
    assert!(count >= 1, "Should find at least one architecture decision");
}

// ============================================================================
// Scenario: Full dispatch index end-to-end
// ============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn test_dispatch_index_flushes_pending_entities() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("Graph init should succeed");

    // Drain any leftover entities
    let _ = codelet_napi::graph::entity_pipeline::take_pending_entities();

    // @step Given entities are queued via the entity pipeline
    let tool_args = serde_json::json!({
        "file_path": "src/main.rs",
        "content": "fn main() {}"
    });
    codelet_napi::graph::entity_pipeline::extract_and_queue_from_tool_call(
        "Write", &tool_args, "test-session", 0,
    );

    // @step When dispatch_index is called
    let result = dispatch::dispatch_index(None).await;

    // @step Then the pending entities are loaded into the graph
    let parsed: serde_json::Value = serde_json::from_str(&result)
        .expect("dispatch_index should return valid JSON");
    assert_eq!(parsed["action"], "index");
    assert_eq!(
        parsed["status"].as_str().unwrap_or(""),
        "indexed",
        "dispatch_index should successfully index entities: {result}"
    );

    // Queue should be empty after indexing
    let remaining = codelet_napi::graph::entity_pipeline::take_pending_entities();
    assert!(remaining.is_empty(), "Queue should be empty after index dispatch");

    // Verify the entities actually made it into the graph
    let stats_json = graph_db_stats().await.expect("Stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let turn_count = stats["nodes"]["Turn"].as_i64().unwrap_or(0);
    let code_entity_count = stats["nodes"]["CodeEntity"].as_i64().unwrap_or(0);
    assert!(turn_count >= 1, "Should have at least 1 Turn node after index, got {turn_count}");
    assert!(code_entity_count >= 1, "Should have at least 1 CodeEntity after index, got {code_entity_count}");
}
