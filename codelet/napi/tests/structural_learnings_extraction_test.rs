//! Tests for structural (non-LLM) learnings extraction from DAG summaries.
//!
//! Feature: spec/features/learnings-extraction-pipeline-session-boundary-analysis.feature
//!
//! Validates that `extract_structural_learnings_from_dag` correctly identifies
//! decisions, conventions, and constraints from DAG text without LLM calls.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::extract_structural_learnings_from_dag;
use codelet_napi::graph::graph_entities::GraphEntity;

// ============================================================================
// Scenario: Extract decisions from DAG summary text
// ============================================================================
#[test]
fn test_extract_decisions_from_dag_text() {
    // @step Given a DAG summary containing decision markers
    let dag_text = r#"
- decided to use dual-graph architecture instead of monolithic approach
- The API layer handles authentication via JWT tokens
- decided to store sessions in Redis with 24h TTL for fast lookups
"#;

    // @step When structural learnings extraction processes the text
    let entities = extract_structural_learnings_from_dag(dag_text);

    // @step Then it should extract Learning nodes with category "decision"
    let decisions: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Learning"
                    && properties.get("category").and_then(|v| v.as_str()) == Some("decision"))
        })
        .collect();

    assert!(
        decisions.len() >= 2,
        "Should extract at least 2 decisions, got {}",
        decisions.len()
    );

    // @step And each Learning node should have slug, title, content, confidence, and timestamps
    for entity in &decisions {
        if let GraphEntity::Node { properties, .. } = entity {
            assert!(properties.contains_key("slug"), "should have slug");
            assert!(properties.contains_key("title"), "should have title");
            assert!(properties.contains_key("content"), "should have content");
            assert!(properties.contains_key("confidence"), "should have confidence");
            assert!(properties.contains_key("firstSeen"), "should have firstSeen");
            assert!(properties.contains_key("lastSeen"), "should have lastSeen");
            assert!(properties.contains_key("mentionCount"), "should have mentionCount");
        }
    }
}

// ============================================================================
// Scenario: Extract conventions from DAG summary text
// ============================================================================
#[test]
fn test_extract_conventions_from_dag_text() {
    // @step Given a DAG summary containing convention markers
    let dag_text = r#"
- Convention: all error messages must use chalk for colored output
- always use const over let when values never change
- never use any type in TypeScript code, use proper types
"#;

    // @step When structural learnings extraction processes the text
    let entities = extract_structural_learnings_from_dag(dag_text);

    // @step Then it should extract Learning nodes with category "convention"
    let conventions: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Learning"
                    && properties.get("category").and_then(|v| v.as_str()) == Some("convention"))
        })
        .collect();

    assert!(
        conventions.len() >= 2,
        "Should extract at least 2 conventions, got {}",
        conventions.len()
    );
}

// ============================================================================
// Scenario: Extract constraints from DAG summary text
// ============================================================================
#[test]
fn test_extract_constraints_from_dag_text() {
    // @step Given a DAG summary containing constraint markers
    let dag_text = r#"
- Constraint: nanograph queries require typed variables, cannot use untyped
- limitation: Lance database does not support concurrent writes from multiple processes
- must not use console.log in production TypeScript source code
"#;

    // @step When structural learnings extraction processes the text
    let entities = extract_structural_learnings_from_dag(dag_text);

    // @step Then it should extract Learning nodes with category "constraint"
    let constraints: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Learning"
                    && properties.get("category").and_then(|v| v.as_str()) == Some("constraint"))
        })
        .collect();

    assert!(
        constraints.len() >= 2,
        "Should extract at least 2 constraints, got {}",
        constraints.len()
    );
}

// ============================================================================
// Scenario: Empty or short text produces no entities
// ============================================================================
#[test]
fn test_empty_text_produces_no_entities() {
    // @step Given an empty DAG summary
    let entities = extract_structural_learnings_from_dag("");
    assert!(entities.is_empty(), "Empty text should produce no entities");

    // @step And a DAG with only short lines
    let entities = extract_structural_learnings_from_dag("short\nlines\nonly");
    assert!(
        entities.is_empty(),
        "Short lines should produce no entities"
    );
}

// ============================================================================
// Scenario: Volume limit enforced at 20 entities
// ============================================================================
#[test]
fn test_volume_limit_enforced() {
    // @step Given a DAG with more than 20 decision/convention/constraint lines
    let mut dag_text = String::new();
    for i in 0..30 {
        dag_text.push_str(&format!(
            "- decided to implement feature {i} using the new approach for better performance\n"
        ));
    }

    // @step When structural learnings extraction processes the text
    let entities = extract_structural_learnings_from_dag(&dag_text);

    // @step Then the result should be capped at 20 entities
    assert!(
        entities.len() <= 20,
        "Should cap at 20 entities, got {}",
        entities.len()
    );
}
