// Feature: spec/features/graphsearch-query-implementations.feature
// Feature: spec/features/graph-merge-upsert-logic.feature
// Feature: spec/features/structural-extractors-zero-cost-indexing.feature
//
// Integration test: Full round-trip — load JSONL into nanograph → query → verify.
// Tests the actual graph database, not mocks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::{
    ensure_graph_db, graph_db_load_jsonl, graph_db_query, graph_db_stats,
    graph_describe_schema, reset_graph_db,
};
use codelet_napi::graph::extractors::{extract_from_file_operation, extract_from_fspec_command};
use codelet_napi::graph::merge::entities_to_jsonl;
use std::sync::Mutex;

// Global mutex — these tests share the GRAPH_DB singleton so must run sequentially
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup an isolated temp directory for a test.
fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    reset_graph_db();
    (guard, temp_dir)
}

/// Bundled queries — same source the handler uses at runtime.
const GRAPH_QUERIES: &str = include_str!("../schemas/graph-queries.gq");

// ============================================================================
// Scenario: Load Concept nodes via JSONL and query them back
// ============================================================================
#[tokio::test]
async fn test_load_concepts_and_search() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given the graph database is initialized
    ensure_graph_db().await.expect("init should succeed");

    // @step When Concept nodes are loaded via JSONL
    let jsonl = r#"{"type":"Concept","data":{"slug":"jwt-authentication","name":"JWT Authentication","category":"technology","summary":"Token-based stateless auth","mentionCount":5,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"session-management","name":"Session Management","category":"pattern","summary":"Server-side session tracking","mentionCount":3,"confidence":"medium","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;

    graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step Then stats reflect the loaded nodes
    let stats_json = graph_db_stats().await.expect("stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let concept_count = stats["nodes"]["Concept"].as_i64().unwrap_or(0);
    assert!(concept_count >= 2, "Should have at least 2 Concept nodes, got {concept_count}");

    // @step And search_concepts query returns matching results
    let params = serde_json::json!({ "query": "JWT" });
    let results = graph_db_query(GRAPH_QUERIES, "search_concepts", Some(&params))
        .await
        .expect("search should succeed");

    let arr = results.as_array().expect("results should be an array");
    assert!(!arr.is_empty(), "Search for 'JWT' should return at least 1 result");

    // Verify the result contains expected fields
    let first = &arr[0];
    assert!(first.get("slug").is_some(), "Result should have slug");
    assert!(first.get("name").is_some(), "Result should have name");
    assert!(first.get("category").is_some(), "Result should have category");
}

// ============================================================================
// Scenario: Load Decision nodes and filter by domain
// ============================================================================
#[tokio::test]
async fn test_load_decisions_and_filter() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given Decision nodes are loaded
    let jsonl = r#"{"type":"Decision","data":{"slug":"use-jwt","title":"Use JWT tokens","rationale":"Statelessness","status":"active","domain":"architecture","decidedAt":"2026-03-19T00:00:00Z","createdAt":"2026-03-19T00:00:00Z"}}
{"type":"Decision","data":{"slug":"use-prettier","title":"Use Prettier","rationale":"Consistency","status":"active","domain":"convention","decidedAt":"2026-03-18T00:00:00Z","createdAt":"2026-03-18T00:00:00Z"}}
{"type":"Decision","data":{"slug":"use-postgres","title":"Use PostgreSQL","rationale":"Reliability","status":"superseded","domain":"architecture","decidedAt":"2026-03-17T00:00:00Z","createdAt":"2026-03-17T00:00:00Z"}}"#;

    graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When all_decisions query is executed
    let results = graph_db_query(GRAPH_QUERIES, "all_decisions", None)
        .await
        .expect("decisions query should succeed");

    let arr = results.as_array().expect("results should be an array");
    assert!(arr.len() >= 3, "Should have at least 3 decisions, got {}", arr.len());

    // @step Then results are filterable by domain
    let arch_decisions: Vec<_> = arr
        .iter()
        .filter(|d| d.get("domain").and_then(|v| v.as_str()) == Some("architecture"))
        .collect();
    assert_eq!(arch_decisions.len(), 2, "Should have 2 architecture decisions");
}

// ============================================================================
// Scenario: Structural extractors produce entities that round-trip through graph
// ============================================================================
#[tokio::test]
async fn test_structural_extractor_round_trip() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given a Write tool call produces entities via extractor (Turn + CodeEntity + edge)
    let entities = extract_from_file_operation("Write", "src/auth/login.rs", "test-session", 0);
    assert_eq!(entities.len(), 3, "Should produce 3 entities: Turn + CodeEntity + Modifies edge");

    // @step When ALL entities (including edges) are converted to JSONL and loaded
    let jsonl = entities_to_jsonl(&entities);
    assert!(!jsonl.is_empty(), "JSONL should not be empty");
    graph_db_load_jsonl(&jsonl).await.expect("load should succeed");

    // @step Then CodeEntity and Turn nodes appear in stats
    let stats_json = graph_db_stats().await.expect("stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let code_entity_count = stats["nodes"]["CodeEntity"].as_i64().unwrap_or(0);
    assert!(code_entity_count >= 1, "Should have at least 1 CodeEntity node, got {code_entity_count}");
    let turn_count = stats["nodes"]["Turn"].as_i64().unwrap_or(0);
    assert!(turn_count >= 1, "Should have at least 1 Turn node (auto-created by extractor), got {turn_count}");

    // @step And the Modifies edge is loaded
    let modifies_count = stats["edges"]["Modifies"].as_i64().unwrap_or(0);
    assert!(modifies_count >= 1, "Should have at least 1 Modifies edge, got {modifies_count}");
}

// ============================================================================
// Scenario: WorkUnit extractor round-trips through graph
// ============================================================================
#[tokio::test]
async fn test_work_unit_extractor_round_trip() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given an Fspec create-story call produces WorkUnit entities
    let entities = extract_from_fspec_command("create-story", "AUTH-001", "User Login", "test-session");
    assert!(!entities.is_empty(), "Should produce WorkUnit entity");

    // @step When entities are loaded into the graph
    let jsonl = entities_to_jsonl(&entities);
    graph_db_load_jsonl(&jsonl).await.expect("load should succeed");

    // @step Then WorkUnit nodes appear in stats
    let stats_json = graph_db_stats().await.expect("stats should succeed");
    let stats: serde_json::Value = serde_json::from_str(&stats_json).unwrap();
    let wu_count = stats["nodes"]["WorkUnit"].as_i64().unwrap_or(0);
    assert!(wu_count >= 1, "Should have at least 1 WorkUnit node");
}

// ============================================================================
// Scenario: RelatesTo edges with strength are queryable
// ============================================================================
#[tokio::test]
async fn test_relates_to_edges_round_trip() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given two Concept nodes and a RelatesTo edge between them
    let jsonl = r#"{"type":"Concept","data":{"slug":"jwt-auth","name":"JWT Auth","category":"technology","summary":"Auth tokens","mentionCount":5,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"session-mgmt","name":"Session Management","category":"pattern","summary":"Session tracking","mentionCount":3,"confidence":"medium","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"edge":"RelatesTo","from":"jwt-auth","to":"session-mgmt","data":{"relationType":"supersedes","strength":0.85,"coOccurrenceCount":3,"firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;

    graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When concept_related query is run for jwt-auth
    let params = serde_json::json!({ "slug": "jwt-auth" });
    let results = graph_db_query(GRAPH_QUERIES, "concept_related", Some(&params))
        .await
        .expect("concept_related query should succeed");

    let arr = results.as_array().expect("results should be an array");
    assert!(!arr.is_empty(), "Should find at least 1 related concept");

    // @step Then the result includes the related concept
    let first = &arr[0];
    let slug = first.get("slug").and_then(|v| v.as_str());
    assert_eq!(slug, Some("session-mgmt"), "Related concept should be session-mgmt");

    // Note: nanograph's query grammar doesn't support edge variable binding,
    // so edge properties (strength, relationType) can't be returned in queries.
    // Edge property filtering happens client-side in the handler.
}

// ============================================================================
// Scenario: concept_neighbors returns related concepts
// ============================================================================
#[tokio::test]
async fn test_concept_neighbors_query() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step Given a concept with neighbors via RelatesTo edges
    let jsonl = r#"{"type":"Concept","data":{"slug":"auth-module","name":"Auth Module","category":"feature","summary":"Authentication","mentionCount":10,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"jwt-lib","name":"JWT Library","category":"library","summary":"JWT tokens","mentionCount":5,"confidence":"high","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"type":"Concept","data":{"slug":"bcrypt-lib","name":"Bcrypt Library","category":"library","summary":"Password hashing","mentionCount":3,"confidence":"medium","firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"edge":"RelatesTo","from":"auth-module","to":"jwt-lib","data":{"relationType":"uses","strength":0.9,"coOccurrenceCount":5,"firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}
{"edge":"RelatesTo","from":"auth-module","to":"bcrypt-lib","data":{"relationType":"uses","strength":0.7,"coOccurrenceCount":3,"firstSeen":"2026-03-01T00:00:00Z","lastSeen":"2026-03-19T00:00:00Z"}}"#;

    graph_db_load_jsonl(jsonl).await.expect("load should succeed");

    // @step When concept_neighbors query is run
    let params = serde_json::json!({ "slug": "auth-module" });
    let results = graph_db_query(GRAPH_QUERIES, "concept_neighbors", Some(&params))
        .await
        .expect("concept_neighbors should succeed");

    let arr = results.as_array().expect("results should be an array");

    // @step Then neighbors include jwt-lib and bcrypt-lib
    assert!(arr.len() >= 2, "Should have at least 2 neighbors, got {}", arr.len());

    let slugs: Vec<&str> = arr
        .iter()
        .filter_map(|r| r.get("slug").and_then(|v| v.as_str()))
        .collect();
    assert!(slugs.contains(&"jwt-lib"), "Should contain jwt-lib neighbor");
    assert!(slugs.contains(&"bcrypt-lib"), "Should contain bcrypt-lib neighbor");
}

// ============================================================================
// Scenario: Empty search returns empty results (not error)
// ============================================================================
#[tokio::test]
async fn test_empty_search_no_error() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    // @step When search_concepts is run on empty graph
    let params = serde_json::json!({ "query": "nonexistent" });
    let results = graph_db_query(GRAPH_QUERIES, "search_concepts", Some(&params))
        .await
        .expect("search on empty graph should not error");

    let arr = results.as_array().expect("results should be an array");
    assert!(arr.is_empty(), "Search on empty graph should return empty array");
}

// ============================================================================
// Scenario: Schema describes all types after init
// ============================================================================
#[tokio::test]
async fn test_schema_description_complete() {
    let (_guard, _temp_dir) = setup_test_env();

    ensure_graph_db().await.expect("init should succeed");

    let desc = graph_describe_schema().await.expect("describe should succeed");

    // Verify all node types
    for nt in &["Concept", "Decision", "CodeEntity", "WorkUnit", "Session", "Turn"] {
        assert!(desc.contains(nt), "Schema should contain node type {nt}");
    }

    // Verify all edge types
    for et in &["Mentions", "Discusses", "Decides", "Implements", "Modifies", "RelatesTo", "Supersedes", "WorksOn", "References", "ContainsTurn"] {
        assert!(desc.contains(et), "Schema should contain edge type {et}");
    }
}
