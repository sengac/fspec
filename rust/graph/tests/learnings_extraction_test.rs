// Feature: spec/features/learnings-extraction-pipeline-session-boundary-analysis.feature
//
// Learnings Extraction Pipeline — Session Boundary Analysis
// Tests for extracting accumulated learnings from session boundaries
// using the Residue methodology structure.
//
// All tests use REAL nanograph databases (tempdir), REAL fixture JSON for LLM responses,
// REAL GraphDatabase::load_entities + dispatch functions for round-trip verification.
// NO mocks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod graph_test_helpers;

use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::learnings_dispatch;
use codelet_graph::learnings_extraction::extract_learnings_from_text;
use graph_test_helpers::{count_nodes, find_node, make_decision};

/// The Learnings graph schema.
const LEARNINGS_SCHEMA: &str = include_str!("../schemas/learnings.pg");

/// Helper: create an isolated Learnings graph database in a temp directory.
async fn setup_learnings_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-learnings.nano");
    GraphDatabase::init(&db_path, LEARNINGS_SCHEMA)
        .await
        .expect("DB init")
}

/// A realistic DAG summary — the kind of text produced by inject_summary compaction.
const REALISTIC_DAG_SUMMARY: &str = r#"
# Session Summary — KGRAPH-019 AST Graph Query Interface

## What was accomplished
Implemented the AST graph query interface for GraphSearch. Fixed multiple issues with
nanograph query syntax that were discovered during testing.

## Key discoveries
- Nanograph queries require typed variables ($fn: Function, not just $fn). Untyped
  variables cause parse errors at the schema validation stage.
- For reverse edge traversal, the destination variable must be bound first with its
  type and filter, then the source resolves automatically via the CSC index.
- Return clauses in .gq files only support $var.prop syntax — no field aliases like
  'slug: $fn.slug' and no string literals.

## Approaches tried
- Tried using untyped edge traversal queries ($src $edge $target) — failed because
  the nanograph parser requires explicit edge type names.
- Tried generic neighbor queries without type annotations — parser rejected them.
- Finally bound destination variable first with type, which resolved the issue.

## Decisions made
- All GraphSearch query files use .gq extension and are bundled via include_str!
- Client-side filtering is used post-query for text search (nanograph has no LIKE operator)
"#;

/// A realistic LLM response — the kind of JSON an LLM would produce when given
/// the LEARNINGS_EXTRACTION_PROMPT and the DAG summary above.
/// Contains 2 learnings, 1 exploration, and 1 constraint (4 entities total).
const REALISTIC_LLM_RESPONSE: &str = r#"```json
{
  "learnings": [
    {
      "slug": "nanograph-requires-typed-variables",
      "title": "Nanograph queries require typed variables",
      "content": "Variables in nanograph .gq queries must be bound to a specific node type like $fn: Function. Untyped variables cause parse errors at the schema validation stage.",
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
      "strategy": "Used $src $edge $target syntax for generic neighbor queries without type annotations",
      "outcome": "failure",
      "failureConstraint": "Nanograph parser requires explicit edge type names and typed variables — untyped queries are rejected at parse time"
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

#[path = "learnings_extraction_test/extract_learnings_explorations_and.rs"]
mod extract_learnings_explorations_and;
#[path = "learnings_extraction_test/learnings_stats_reflect_loaded.rs"]
mod learnings_stats_reflect_loaded;
