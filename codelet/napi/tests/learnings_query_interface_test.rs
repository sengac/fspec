// Feature: spec/features/learnings-graph-query-interface.feature
//
// Learnings Graph Query Interface
// Tests for Learnings-specific query actions (LearningsSearch, LearningsDecisions,
// LearningsStats, LearningsRelated) routed through the GraphSearch tool infrastructure.
//
// Each test populates an isolated Learnings graph database with known data,
// then exercises the dispatch functions directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::learnings_dispatch;
use serde_json::Value;

/// The Learnings schema.
const LEARNINGS_SCHEMA: &str = include_str!("../../graph/schemas/learnings.pg");

/// Helper: create a Learnings graph database pre-loaded with test data.
async fn setup_test_learnings_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-learnings.nano");
    let db = GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("DB init");

    // Load test data: 3 learnings, 2 decisions, 1 exploration, 1 convention, 1 code pattern
    let jsonl = r#"{"type":"Learning","data":{"slug":"learn-nanograph-edge-types","title":"Nanograph queries require explicit edge type names","content":"When writing nanograph PG queries, you must name each edge type explicitly. Generic traversal with untyped variables is not supported.","category":"discovery","confidence":"high","projectPath":"/home/user/projects/fspec","firstSeen":"2026-03-20T10:00:00Z","lastSeen":"2026-03-22T10:00:00Z","mentionCount":3,"tags":["nanograph","queries"]}}
{"type":"Learning","data":{"slug":"learn-error-handling-rust","title":"Rust error handling with anyhow","content":"Use anyhow::Result for application-level errors and thiserror for library-level errors. Never unwrap in production code.","category":"convention","confidence":"high","projectPath":"/home/user/projects/fspec","firstSeen":"2026-03-18T10:00:00Z","lastSeen":"2026-03-21T10:00:00Z","mentionCount":5,"tags":["rust","error-handling"]}}
{"type":"Learning","data":{"slug":"learn-batch-loading","title":"Batch loading prevents Lance version amplification","content":"Loading entities one at a time causes Lance version amplification. Always batch-collect and load in a single operation.","category":"pattern","confidence":"high","projectPath":"/home/user/projects/fspec","firstSeen":"2026-03-19T10:00:00Z","lastSeen":"2026-03-22T10:00:00Z","mentionCount":2,"tags":["performance","lance"]}}
{"type":"Decision","data":{"slug":"dec-dual-graph-arch","title":"Use dual-graph architecture","rationale":"Single monolithic graph consumed 7.6GB. Splitting into AST (code structure) and Learnings (knowledge) provides targeted queries with <15MB total.","status":"active","domain":"architecture","alternatives":"Single graph with selective indexing; Triple-store approach","decidedAt":"2026-03-22T02:00:00Z","createdAt":"2026-03-22T02:00:00Z"}}
{"type":"Decision","data":{"slug":"dec-session-boundary-extraction","title":"Extract at session boundaries not per-turn","rationale":"Per-turn extraction caused graph bloat (7.6GB for 727 turns). Session-boundary extraction produces 5-20 entities per session.","status":"active","domain":"architecture","alternatives":"Per-turn extraction; Periodic batch extraction every N turns","decidedAt":"2026-03-22T03:00:00Z","createdAt":"2026-03-22T03:00:00Z"}}
{"type":"Exploration","data":{"slug":"exp-tree-sitter-direct","title":"Direct tree-sitter API for AST parsing","strategy":"Use tree-sitter API directly for parsing code files","outcome":"abandoned","failureConstraint":"tree-sitter API requires language-specific grammar loading. ast-grep wraps this with pattern matching.","survivingStructure":"Pattern-based extraction approach survived","sessionSlug":"session-001","createdAt":"2026-03-22T04:00:00Z"}}
{"type":"Convention","data":{"slug":"conv-separate-dispatch-files","title":"Separate dispatch files per graph","description":"Each graph type (agent-memory, AST, learnings) must have its own dispatch file for separation of concerns.","enforcement":"mandatory","scope":"project","language":"rust","createdAt":"2026-03-22T02:30:00Z","updatedAt":"2026-03-22T02:30:00Z"}}
{"type":"CodePattern","data":{"slug":"pat-registry-singleton","name":"Registry singleton pattern","description":"Use a global Mutex-wrapped HashMap as a lazy-initialized registry for named database instances.","patternType":"structural","exampleFile":"codelet/napi/src/graph/registry.rs","language":"rust","firstSeen":"2026-03-22T02:00:00Z","lastSeen":"2026-03-22T06:00:00Z","usageCount":3}}
{"edge":"Discovered","from":"exp-tree-sitter-direct","to":"learn-batch-loading","data":{"extractedAt":"2026-03-22T04:30:00Z"}}
{"edge":"RelatesTo","from":"learn-nanograph-edge-types","to":"learn-error-handling-rust","data":{"strength":0.6,"relationType":"similar_to","firstSeen":"2026-03-22T05:00:00Z","lastSeen":"2026-03-22T05:00:00Z"}}
{"edge":"RelatesTo","from":"learn-error-handling-rust","to":"learn-batch-loading","data":{"strength":0.4,"relationType":"composes","firstSeen":"2026-03-22T05:00:00Z","lastSeen":"2026-03-22T05:00:00Z"}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: Search for a learning by text using LearningsSearch action
// ============================================================================
#[tokio::test]
async fn test_learnings_search_by_text() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the Learnings graph contains Learning nodes with various categories and domains
    let db = setup_test_learnings_db(temp_dir.path()).await;

    // @step When I search learnings with query "nanograph queries require explicit edge type names"
    let result = learnings_dispatch::dispatch_learnings_search(
        &db,
        "nanograph queries require explicit edge type names",
        None,
        None,
    )
    .await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive matching Learning nodes with slug, category, confidence, domain, and session origin
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(!results.is_empty(), "Should find at least one result");

    let first = &results[0];
    assert_eq!(
        first.get("slug").and_then(|v| v.as_str()),
        Some("learn-nanograph-edge-types")
    );
    assert!(first.get("category").is_some(), "Result should have category");
    assert!(
        first.get("confidence").is_some(),
        "Result should have confidence"
    );
}

// ============================================================================
// Scenario: Query decisions filtered by domain and status using LearningsDecisions action
// ============================================================================
#[tokio::test]
async fn test_learnings_decisions_filtered_by_domain_and_status() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the Learnings graph contains Decision nodes with domain, status, rationale, and alternatives
    let db = setup_test_learnings_db(temp_dir.path()).await;

    // @step When I query decisions with domain "architecture" and status "active"
    let result = learnings_dispatch::dispatch_learnings_decisions(
        &db,
        Some("architecture"),
        Some("active"),
    )
    .await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive only active architectural decisions with their rationale and alternatives
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(
        results.len() >= 2,
        "Should find at least 2 active architecture decisions"
    );

    // All results should have domain=architecture, status=active
    for decision in results {
        assert_eq!(
            decision.get("domain").and_then(|v| v.as_str()),
            Some("architecture"),
            "All decisions should have domain=architecture"
        );
        assert_eq!(
            decision.get("status").and_then(|v| v.as_str()),
            Some("active"),
            "All decisions should have status=active"
        );
        assert!(
            decision.get("rationale").is_some(),
            "Decisions should include rationale"
        );
    }
}

// ============================================================================
// Scenario: Get Learnings graph statistics using LearningsStats action
// ============================================================================
#[tokio::test]
async fn test_learnings_stats() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the Learnings graph contains nodes of type Learning, Exploration, Convention, Decision, and CodePattern
    let db = setup_test_learnings_db(temp_dir.path()).await;

    // @step When I request Learnings graph statistics
    let result = learnings_dispatch::dispatch_learnings_stats(&db).await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive node counts per type and total edge counts
    let nodes = parsed.get("nodes").expect("Should have nodes object");
    assert!(
        nodes.get("Learning").and_then(|v| v.as_u64()).unwrap_or(0) >= 3,
        "Should have at least 3 Learning nodes"
    );
    assert!(
        nodes.get("Decision").and_then(|v| v.as_u64()).unwrap_or(0) >= 2,
        "Should have at least 2 Decision nodes"
    );
    assert!(
        nodes
            .get("Exploration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "Should have at least 1 Exploration node"
    );
    assert!(
        nodes
            .get("Convention")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "Should have at least 1 Convention node"
    );
    assert!(
        nodes
            .get("CodePattern")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1,
        "Should have at least 1 CodePattern node"
    );

    let edges = parsed.get("edges").expect("Should have edges object");
    assert!(
        edges.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "Should have at least 1 total edge"
    );
}

// ============================================================================
// Scenario: Find related learnings by topic using LearningsRelated action
// ============================================================================
#[tokio::test]
async fn test_learnings_related_by_topic() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the Learnings graph contains Learning nodes connected by RelatesTo edges with strength values
    let db = setup_test_learnings_db(temp_dir.path()).await;

    // @step When I search for learnings related to topic "error handling"
    let result = learnings_dispatch::dispatch_learnings_related(
        &db,
        "error handling",
        None,
        None,
    )
    .await;
    let parsed: Value = serde_json::from_str(&result).expect("valid JSON");

    // @step Then I should receive related Learning nodes sorted by relevance with strength and relation type
    let results = parsed
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    assert!(
        !results.is_empty(),
        "Should find at least one related learning"
    );

    // Check that related results have the expected metadata
    for related in results {
        assert!(
            related.get("slug").is_some(),
            "Related result should have slug"
        );
    }
}
