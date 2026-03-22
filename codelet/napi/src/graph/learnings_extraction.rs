//! Learnings Extraction Pipeline
//!
//! Extracts structured knowledge from session text (DAG summaries,
//! conversation history) using the Residue methodology categories.
//!
//! This pipeline:
//! - Operates at session boundaries (compaction, work unit completion)
//! - Produces 5-20 entities per extraction (not hundreds)
//! - Targets the Learnings graph (registry::LEARNINGS_GRAPH)
//! - Uses Residue categories: Learning, Exploration, Constraint
//!
//! Production currently uses structural extraction (`extract_structural_learnings_from_dag`
//! in mod.rs) for zero-cost extraction without LLM calls.
//! This module provides the LLM-based extraction pipeline (`extract_learnings_from_text`)
//! which produces richer results when an LLM is available. The pipeline accepts
//! pre-computed LLM responses for testability.

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Map, Value};
use tracing::warn;

use super::graph_entities::GraphEntity;
use super::llm_response_parser::extract_json_from_response;

/// Maximum entities allowed per extraction to prevent graph bloat.
const MAX_ENTITIES_PER_EXTRACTION: usize = 20;

/// Result of a learnings extraction operation.
#[derive(Debug)]
pub struct LearningsExtractionResult {
    /// All entities extracted from the text.
    pub entities: Vec<GraphEntity>,
    /// Number of learnings extracted.
    pub learning_count: usize,
    /// Number of explorations extracted.
    pub exploration_count: usize,
    /// Number of constraints extracted (Learning nodes with category "constraint").
    pub constraint_count: usize,
}

/// The Residue methodology system prompt for learnings extraction.
///
/// Instructs the LLM to produce structured entities in three categories:
/// Learnings, Explorations, and Constraints.
pub const LEARNINGS_EXTRACTION_PROMPT: &str = r#"You are a knowledge extraction engine following the Residue methodology.

Given session text, extract ONLY what is EXPLICITLY discussed. Never infer or speculate.

Output a JSON object with three arrays:

{
  "learnings": [
    {
      "slug": "kebab-case-unique-id",
      "title": "Short descriptive title",
      "content": "Detailed explanation of the knowledge gained",
      "category": "convention|pattern|anti_pattern|decision|discovery|constraint|reformulation",
      "confidence": "high|medium|low"
    }
  ],
  "explorations": [
    {
      "slug": "kebab-case-unique-id",
      "title": "What was tried",
      "strategy": "The approach taken",
      "outcome": "success|partial|failure|abandoned",
      "failureConstraint": "Why it failed (if failure/abandoned, otherwise null)"
    }
  ],
  "constraints": [
    {
      "slug": "kebab-case-unique-id",
      "title": "Short constraint title",
      "content": "Hard fact or limitation discovered",
      "category": "constraint",
      "confidence": "high"
    }
  ]
}

Rules:
- Produce 5-20 entities TOTAL across all three arrays
- Use kebab-case for all slugs
- Constraints are stored as Learning nodes with category "constraint"
- Set confidence: "high" (explicitly stated), "medium" (contextually clear), "low" (ambiguous)
- Only include what IS in the text, not what SHOULD be
"#;

/// Raw LLM response structure for deserialization.
#[derive(Debug, Deserialize)]
struct RawExtractionResponse {
    #[serde(default)]
    learnings: Vec<RawLearning>,
    #[serde(default)]
    explorations: Vec<RawExploration>,
    #[serde(default)]
    constraints: Vec<RawLearning>,
}

#[derive(Debug, Deserialize)]
struct RawLearning {
    slug: Option<String>,
    title: Option<String>,
    content: Option<String>,
    category: Option<String>,
    confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawExploration {
    slug: Option<String>,
    title: Option<String>,
    strategy: Option<String>,
    outcome: Option<String>,
    #[serde(rename = "failureConstraint")]
    failure_constraint: Option<String>,
}

/// Extract learnings from text using a pre-computed LLM response.
///
/// This function is testable: pass `Some(response)` with a mock LLM response
/// or `None` to simulate LLM unavailability (returns Err).
///
/// In production, the caller invokes the LLM with `LEARNINGS_EXTRACTION_PROMPT`
/// and passes the response string here.
pub fn extract_learnings_from_text(
    _source_text: &str,
    llm_response: Option<&str>,
) -> Result<LearningsExtractionResult, String> {
    let response = llm_response
        .ok_or_else(|| "LLM unavailable: no response provided".to_string())?;

    let json_str = extract_json_from_response(response);

    let raw: RawExtractionResponse = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse LLM response: {e}"))?;

    let now = Utc::now().to_rfc3339();
    let mut entities = Vec::new();
    let mut learning_count = 0;
    let mut exploration_count = 0;
    let mut constraint_count = 0;

    // Process learnings
    for raw_learning in &raw.learnings {
        if let Some(entity) = validate_learning(raw_learning, &now) {
            entities.push(entity);
            learning_count += 1;
        }
    }

    // Process constraints (stored as Learning nodes with category "constraint")
    for raw_constraint in &raw.constraints {
        if let Some(entity) = validate_learning(raw_constraint, &now) {
            entities.push(entity);
            constraint_count += 1;
        }
    }

    // Process explorations
    for raw_exploration in &raw.explorations {
        if let Some(entity) = validate_exploration(raw_exploration, &now) {
            entities.push(entity);
            exploration_count += 1;
        }
    }

    // Enforce volume constraint
    if entities.len() > MAX_ENTITIES_PER_EXTRACTION {
        warn!(
            count = entities.len(),
            max = MAX_ENTITIES_PER_EXTRACTION,
            "Truncating learnings extraction to volume limit"
        );
        entities.truncate(MAX_ENTITIES_PER_EXTRACTION);
    }

    Ok(LearningsExtractionResult {
        entities,
        learning_count,
        exploration_count,
        constraint_count,
    })
}

/// Valid categories for Learning nodes.
const VALID_LEARNING_CATEGORIES: &[&str] = &[
    "convention",
    "pattern",
    "anti_pattern",
    "decision",
    "discovery",
    "constraint",
    "reformulation",
];

/// Valid outcomes for Exploration nodes.
const VALID_EXPLORATION_OUTCOMES: &[&str] =
    &["success", "partial", "failure", "abandoned"];

/// Validate and convert a raw learning into a GraphEntity.
fn validate_learning(raw: &RawLearning, now: &str) -> Option<GraphEntity> {
    let slug = raw.slug.as_deref().filter(|s| !s.is_empty())?;
    let title = raw.title.as_deref().filter(|s| !s.is_empty())?;
    let content = raw.content.as_deref().filter(|s| !s.is_empty())?;
    let category = raw.category.as_deref().filter(|s| !s.is_empty())?;

    if !VALID_LEARNING_CATEGORIES.contains(&category) {
        warn!(slug, category, "Invalid learning category — skipping");
        return None;
    }

    let confidence = raw
        .confidence
        .as_deref()
        .filter(|c| ["high", "medium", "low"].contains(c))
        .unwrap_or("medium");

    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(slug.to_string()));
    props.insert("title".to_string(), Value::String(title.to_string()));
    props.insert("content".to_string(), Value::String(content.to_string()));
    props.insert("category".to_string(), Value::String(category.to_string()));
    props.insert(
        "confidence".to_string(),
        Value::String(confidence.to_string()),
    );
    props.insert("firstSeen".to_string(), Value::String(now.to_string()));
    props.insert("lastSeen".to_string(), Value::String(now.to_string()));
    props.insert("mentionCount".to_string(), Value::Number(1.into()));

    Some(GraphEntity::Node {
        node_type: "Learning".to_string(),
        slug: slug.to_string(),
        properties: props,
    })
}

/// Validate and convert a raw exploration into a GraphEntity.
fn validate_exploration(raw: &RawExploration, now: &str) -> Option<GraphEntity> {
    let slug = raw.slug.as_deref().filter(|s| !s.is_empty())?;
    let title = raw.title.as_deref().filter(|s| !s.is_empty())?;
    let strategy = raw.strategy.as_deref().filter(|s| !s.is_empty())?;
    let outcome = raw.outcome.as_deref().filter(|s| !s.is_empty())?;

    if !VALID_EXPLORATION_OUTCOMES.contains(&outcome) {
        warn!(slug, outcome, "Invalid exploration outcome — skipping");
        return None;
    }

    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(slug.to_string()));
    props.insert("title".to_string(), Value::String(title.to_string()));
    props.insert("strategy".to_string(), Value::String(strategy.to_string()));
    props.insert("outcome".to_string(), Value::String(outcome.to_string()));
    props.insert("createdAt".to_string(), Value::String(now.to_string()));

    if let Some(fc) = raw.failure_constraint.as_deref().filter(|s| !s.is_empty()) {
        props.insert(
            "failureConstraint".to_string(),
            Value::String(fc.to_string()),
        );
    }

    Some(GraphEntity::Node {
        node_type: "Exploration".to_string(),
        slug: slug.to_string(),
        properties: props,
    })
}

// extract_json_from_response moved to llm_response_parser.rs (DRY)
