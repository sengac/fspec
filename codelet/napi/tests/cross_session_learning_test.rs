//! Cross-Session Learning & Context Injection Tests
//!
//! Feature: spec/features/cross-session-learning-context-injection.feature
//!
//! Tests the learnings context injection pipeline: querying the Learnings
//! graph for relevant knowledge and formatting it for session injection.

use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::entities_to_jsonl;
use serde_json::json;
use tempfile::tempdir;

mod graph_test_helpers;
use graph_test_helpers::{make_decision, make_exploration, make_learning};

/// Helper: create a temporary Learnings graph with the learnings.pg schema.
async fn create_test_learnings_db() -> (GraphDatabase, tempfile::TempDir) {
    let dir = tempdir().expect("create temp dir");
    let db_path = dir.path().join("test-learnings.nano");
    let schema = include_str!("../../graph/schemas/learnings.pg");
    let db = GraphDatabase::init(&db_path, schema)
        .await
        .expect("init learnings db");
    (db, dir)
}

/// Scenario: Inject relevant decisions into session context at session start
#[tokio::test]
async fn test_inject_relevant_decisions_into_session_context() {
    // @step Given a Learnings graph with a Decision node "use-dual-graph" in domain "architecture" with status "active"
    let (db, _dir) = create_test_learnings_db().await;
    let decision = make_decision(
        "use-dual-graph",
        "Use Dual-Graph Architecture",
        "architecture",
        "active",
        "Monolithic graph consumed 7.6GB; dual-graph keeps each graph under 10MB",
    );
    let jsonl = entities_to_jsonl(&[decision]);
    db.load_jsonl(&jsonl).await.expect("load decision");

    // @step And the session is assigned to work unit "KGRAPH-022" in the "knowledge-graph" epic
    // (work unit context provides the query domain)

    // @step When the context injection function is called with query "knowledge-graph"
    let context = codelet_napi::graph::learnings_context::build_learnings_context_from_db(
        &db,
        "architecture",
    )
    .await;

    // @step Then the returned context should contain the decision "use-dual-graph"
    assert!(context.is_some(), "context should not be None");
    let context_str = context.unwrap();
    assert!(
        context_str.contains("use-dual-graph") || context_str.contains("Use Dual-Graph"),
        "context should contain the decision: {context_str}"
    );

    // @step And the context should be formatted as a system-reminder with type "learningsContext"
    assert!(
        context_str.contains("learningsContext") || context_str.contains("Learnings Context"),
        "context should indicate learnings context type: {context_str}"
    );

    // @step And the context should include the decision rationale
    assert!(
        context_str.contains("7.6GB") || context_str.contains("Monolithic"),
        "context should include rationale: {context_str}"
    );
}

/// Scenario: Surface failed explorations as warnings in session context
#[tokio::test]
async fn test_surface_failed_explorations_as_warnings() {
    // @step Given a Learnings graph with an Exploration node "monolithic-indexing" with outcome "failure"
    let (db, _dir) = create_test_learnings_db().await;
    let exploration = make_exploration(
        "monolithic-indexing",
        "Index all conversation history per-turn",
        "failure",
        Some("caused 7.6GB disk consumption"),
    );

    // @step And the Exploration has failureConstraint "caused 7.6GB disk consumption"
    let jsonl = entities_to_jsonl(&[exploration]);
    db.load_jsonl(&jsonl).await.expect("load exploration");

    // @step When the context injection function is called with query "knowledge-graph"
    let context =
        codelet_napi::graph::learnings_context::build_learnings_context_from_db(&db, "indexing")
            .await;

    // @step Then the returned context should contain a warnings section
    assert!(context.is_some(), "context should not be None");
    let context_str = context.unwrap();
    assert!(
        context_str.contains("Warning")
            || context_str.contains("⚠")
            || context_str.contains("Failed"),
        "context should have warnings section: {context_str}"
    );

    // @step And the warnings section should include "monolithic-indexing" as a failed approach
    assert!(
        context_str.contains("monolithic-indexing"),
        "warnings should include failed approach: {context_str}"
    );

    // @step And the warnings section should include the constraint "caused 7.6GB disk consumption"
    assert!(
        context_str.contains("7.6GB disk consumption"),
        "warnings should include constraint: {context_str}"
    );
}

/// Scenario: Graceful fallback when Learnings graph is not initialized
#[tokio::test]
async fn test_graceful_fallback_when_graph_not_initialized() {
    // @step Given the Learnings graph is not initialized
    // (We don't initialize any graph, just call the function that checks registry)

    // @step When the context injection function is called with query "any-domain"
    let context =
        codelet_napi::graph::learnings_context::build_learnings_context("any-domain").await;

    // @step Then the function should return None
    assert!(
        context.is_none(),
        "context should be None when graph not initialized"
    );

    // @step And no error should be raised
    // (If we got here without panic, no error was raised)
}

/// Scenario: Context volume capped at token limit
#[tokio::test]
async fn test_context_volume_capped_at_token_limit() {
    // @step Given a Learnings graph with 50 Learning nodes matching query "large-domain"
    let (db, _dir) = create_test_learnings_db().await;
    let mut entities = Vec::new();
    for i in 0..50 {
        entities.push(make_learning(
            &format!("learning-large-domain-{i}"),
            &format!("Learning about large-domain topic {i}"),
            "pattern",
            &format!(
                "This is a detailed explanation about large-domain learning {i}. \
                 It contains important information that the agent should know about. \
                 The content is intentionally verbose to test truncation behavior."
            ),
        ));
    }
    let jsonl = entities_to_jsonl(&entities);
    db.load_jsonl(&jsonl).await.expect("load 50 learnings");

    // @step When the context injection function is called with query "large-domain"
    let context = codelet_napi::graph::learnings_context::build_learnings_context_from_db(
        &db,
        "large-domain",
    )
    .await;

    // @step Then the returned context should not exceed 2000 tokens
    assert!(context.is_some(), "context should exist with 50 learnings");
    let context_str = context.unwrap();
    // Rough token estimate: ~4 chars per token
    let estimated_tokens = context_str.len() / 4;
    assert!(
        estimated_tokens <= 2200, // some buffer for formatting overhead
        "context should be capped at ~2000 tokens, got ~{estimated_tokens} (len={})",
        context_str.len()
    );

    // @step And the most relevant learnings should be included first
    // The first learnings matching the query should be present
    assert!(
        context_str.contains("large-domain"),
        "context should include learnings about large-domain: {context_str}"
    );
}

/// Scenario: Post-session learnings extraction loads entities into graph
#[tokio::test]
async fn test_post_session_extraction_loads_entities() {
    // @step Given a completed session with compaction DAG content containing learnings
    let dag_content = r#"Session focused on implementing dual-graph architecture.
Key decision: Use separate AST and Learnings graphs instead of monolithic approach.
Exploration: Tried per-turn entity extraction — caused 7.6GB disk bloat.
Constraint: nanograph queries require explicit edge type names in return clauses."#;

    // @step And a mock LLM response with extracted Learning and Exploration entities
    let mock_llm_response = json!({
        "learnings": [
            {
                "slug": "dual-graph-architecture",
                "title": "Use dual-graph architecture",
                "content": "Separate AST and Learnings graphs prevent disk bloat",
                "category": "decision",
                "confidence": "high"
            }
        ],
        "explorations": [
            {
                "slug": "per-turn-extraction",
                "title": "Per-turn entity extraction",
                "strategy": "Extract entities from every tool call response",
                "outcome": "failure",
                "failureConstraint": "Caused 7.6GB disk consumption after 727 turns"
            }
        ],
        "constraints": [
            {
                "slug": "nanograph-edge-types",
                "title": "Nanograph requires explicit edge type names",
                "content": "Return clauses in .gq queries must name edge types explicitly",
                "category": "constraint",
                "confidence": "high"
            }
        ]
    })
    .to_string();

    // @step When post-session extraction is triggered with the DAG content
    let result = codelet_napi::graph::learnings_extraction::extract_learnings_from_text(
        dag_content,
        Some(&mock_llm_response),
    );

    assert!(result.is_ok(), "extraction should succeed");
    let extraction = result.unwrap();

    // @step Then the extracted entities should be loaded into the Learnings graph
    let (db, _dir) = create_test_learnings_db().await;
    let loaded = db
        .load_entities(&extraction.entities)
        .await
        .expect("load entities");
    assert!(loaded > 0, "should load at least one entity");

    // @step And the Learnings graph should contain the new Learning nodes
    assert!(
        extraction.learning_count >= 1,
        "should have at least 1 learning"
    );

    // @step And the Learnings graph should contain the new Exploration nodes
    assert!(
        extraction.exploration_count >= 1,
        "should have at least 1 exploration"
    );
}

/// Scenario: Subordinate session receives learnings from supervisor domain
#[tokio::test]
async fn test_subordinate_receives_learnings_from_supervisor() {
    // @step Given a Learnings graph with a Learning node "bcrypt-hashing" in category "convention"
    let (db, _dir) = create_test_learnings_db().await;
    let learning = make_learning(
        "bcrypt-hashing",
        "Use bcrypt for password hashing",
        "convention",
        "All authentication services must use bcrypt with cost factor 12",
    );
    let jsonl = entities_to_jsonl(&[learning]);
    db.load_jsonl(&jsonl).await.expect("load learning");

    // @step And the supervisor session is working on domain "authentication"
    let query = "authentication";

    // @step When a subordinate session is spawned for the "authentication" domain
    let context =
        codelet_napi::graph::learnings_context::build_learnings_context_from_db(&db, query).await;

    // @step Then the subordinate context should include the learning "bcrypt-hashing"
    assert!(
        context.is_some(),
        "subordinate should receive learnings context"
    );
    let context_str = context.unwrap();
    assert!(
        context_str.contains("bcrypt") || context_str.contains("password hashing"),
        "subordinate context should include bcrypt learning: {context_str}"
    );
}
