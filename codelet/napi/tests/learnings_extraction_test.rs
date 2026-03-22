// Feature: spec/features/learnings-extraction-pipeline-session-boundary-analysis.feature
//
// Learnings Extraction Pipeline — Session Boundary Analysis
// Tests for extracting accumulated learnings from session boundaries
// using LLM analysis and the Residue methodology structure.
//
// Each test uses an isolated Learnings graph database with mock LLM responses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::GraphEntity;
use codelet_napi::graph::learnings_extraction::extract_learnings_from_text;

/// The Learnings graph schema.
const LEARNINGS_SCHEMA: &str = include_str!("../schemas/learnings.pg");

/// Helper: create an isolated Learnings graph database.
async fn setup_learnings_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-learnings.nano");
    GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("DB init")
}

/// Simulated LLM response following the Residue methodology structure.
const MOCK_LLM_RESPONSE: &str = r#"```json
{
  "learnings": [
    {
      "slug": "nanograph-query-syntax-requires-typed-vars",
      "title": "Nanograph query syntax requires typed variables",
      "content": "Variables in nanograph .gq queries must be bound to a specific node type. Untyped variables cause parse errors.",
      "category": "discovery",
      "confidence": "high"
    },
    {
      "slug": "reverse-traversal-bind-destination-first",
      "title": "Reverse traversal requires binding destination first",
      "content": "For reverse edge traversal in nanograph, bind the destination variable first with type and filter, then the source is resolved via CSC index.",
      "category": "pattern",
      "confidence": "high"
    }
  ],
  "explorations": [
    {
      "slug": "exploration-untyped-edge-queries",
      "title": "Tried untyped edge traversal queries",
      "strategy": "Used $src $edge $target syntax for generic neighbor queries",
      "outcome": "failure",
      "failureConstraint": "Nanograph parser requires explicit edge type names and typed variables"
    }
  ],
  "constraints": [
    {
      "slug": "nanograph-no-return-aliases",
      "title": "Nanograph return clauses cannot use aliases or literals",
      "content": "Return clauses in .gq files only support $var.prop syntax — no field aliases like 'slug: $fn.slug' and no string literals.",
      "category": "constraint",
      "confidence": "high"
    }
  ]
}
```"#;

// ============================================================================
// Scenario: Extract learnings from a compaction DAG summary
// ============================================================================
#[tokio::test]
async fn test_extract_learnings_from_dag_summary() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given a compaction DAG summary text describing work done in a session
    let summary_text = r#"
## Session Summary
Worked on KGRAPH-019 AST Graph Query Interface. Fixed nanograph query syntax issues:
- Discovered that nanograph queries require typed variables ($fn: Function, not just $fn)
- Edge traversal needs explicit edge names (contains, calls) not generic $edge variables
- Return clauses only support $var.prop syntax, no aliases or literals
- For reverse traversal, bind the destination variable first
"#;

    // @step And the Learnings graph database is initialized
    let db = setup_learnings_db(temp_dir.path()).await;

    // @step When the learnings extraction pipeline processes the summary text
    let result = extract_learnings_from_text(
        summary_text,
        Some(MOCK_LLM_RESPONSE),
    );

    // @step Then Learning nodes should be created in the Learnings graph database
    assert!(result.is_ok(), "Extraction should succeed");
    let extraction = result.unwrap();
    assert!(
        !extraction.entities.is_empty(),
        "Should produce at least one entity"
    );

    let learning_count = extraction
        .entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Learning"))
        .count();
    assert!(learning_count >= 1, "Should produce Learning nodes");

    // @step And each Learning node should have a title, category, confidence, and content
    for entity in &extraction.entities {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "Learning" {
                assert!(
                    properties.contains_key("title"),
                    "Learning should have title"
                );
                assert!(
                    properties.contains_key("category"),
                    "Learning should have category"
                );
                assert!(
                    properties.contains_key("confidence"),
                    "Learning should have confidence"
                );
                assert!(
                    properties.contains_key("content"),
                    "Learning should have content"
                );
            }
        }
    }

    // @step And the extraction should produce between 1 and 20 entities
    let total_entities = extraction.entities.len();
    assert!(
        total_entities >= 1 && total_entities <= 20,
        "Should produce 1-20 entities, got {}",
        total_entities
    );

    // Verify we can load them into the DB
    let load_result = db.load_entities(&extraction.entities).await;
    assert!(load_result.is_ok(), "Loading entities into graph should succeed");
}

// ============================================================================
// Scenario: Extract explorations and constraints from session history
// ============================================================================
#[tokio::test]
async fn test_extract_explorations_and_constraints() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given a session conversation history describing multiple approaches tried
    let session_text = r#"
Tried three approaches for generic neighbor queries:
1. Untyped variables $src $edge $target — parse error
2. Typed destination $src calls $fn: Function — also parse error
3. Bind destination first, then source resolved via reverse traversal — works!
Constraint: nanograph return clauses cannot include literal values.
"#;

    // @step And the Learnings graph database is initialized
    let db = setup_learnings_db(temp_dir.path()).await;

    // @step When the learnings extraction pipeline processes the session text
    let result = extract_learnings_from_text(
        session_text,
        Some(MOCK_LLM_RESPONSE),
    );
    assert!(result.is_ok(), "Extraction should succeed");
    let extraction = result.unwrap();

    // @step Then Exploration nodes should be created for approaches tried with outcome and status
    let exploration_count = extraction
        .entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Exploration"))
        .count();
    assert!(
        exploration_count >= 1,
        "Should produce at least one Exploration node"
    );

    // Verify Exploration nodes have required fields
    for entity in &extraction.entities {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "Exploration" {
                assert!(properties.contains_key("title"), "Exploration should have title");
                assert!(properties.contains_key("strategy"), "Exploration should have strategy");
                assert!(properties.contains_key("outcome"), "Exploration should have outcome");
            }
        }
    }

    // @step And Constraint nodes should be created for hard facts discovered
    // Constraints are stored as Learning nodes with category "constraint"
    let constraint_count = extraction
        .entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Learning" && properties.get("category").and_then(|v| v.as_str()) == Some("constraint"))
        })
        .count();
    assert!(
        constraint_count >= 1,
        "Should produce at least one constraint Learning node"
    );

    // Verify we can load them into the DB
    let load_result = db.load_entities(&extraction.entities).await;
    assert!(load_result.is_ok(), "Loading entities into graph should succeed");
}

// ============================================================================
// Scenario: Graceful failure when LLM is unavailable
// ============================================================================
#[tokio::test]
async fn test_graceful_failure_when_llm_unavailable() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given the Learnings graph database is initialized
    let db = setup_learnings_db(temp_dir.path()).await;

    // @step And the LLM provider returns an error
    // Pass None as mock response to simulate LLM unavailability

    // @step When the learnings extraction pipeline attempts to process text
    let result = extract_learnings_from_text(
        "Some session text to process",
        None, // No LLM response available
    );

    // @step Then no entities should be written to the Learnings graph
    assert!(result.is_err(), "Should return an error result");

    // Verify the graph is still empty
    let stats = db.stats().unwrap();
    let total_nodes: u64 = stats
        .get("nodes")
        .and_then(|v| v.as_object())
        .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum())
        .unwrap_or(0);
    assert_eq!(total_nodes, 0, "Graph should remain empty after failure");

    // @step And the pipeline should return an error result without panicking
    // If we got here, the pipeline didn't panic — test passes
}
