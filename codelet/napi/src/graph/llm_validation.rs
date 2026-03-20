//! LLM Extraction Validation
//!
//! Validates extracted concepts, decisions, and relations from LLM responses.
//! Separated from llm_extraction.rs for the 300-line file size limit.
//!
//! Feature: spec/features/llm-concept-extraction.feature

use super::extractors::{GraphEntity, turn_slug};
use serde_json::Map;
use tracing::warn;

/// Valid concept categories from the agent-memory schema.
const VALID_CONCEPT_CATEGORIES: &[&str] = &[
    "architecture",
    "convention",
    "decision",
    "dependency",
    "domain_term",
    "error_class",
    "feature",
    "library",
    "pattern",
    "person",
    "platform",
    "process",
    "technology",
    "tool",
];

/// Valid decision domains from the agent-memory schema.
const VALID_DECISION_DOMAINS: &[&str] = &[
    "architecture",
    "convention",
    "dependency",
    "deployment",
    "design",
    "implementation",
    "process",
    "testing",
];

/// Valid relation types from the agent-memory schema.
const VALID_RELATION_TYPES: &[&str] = &[
    "causes",
    "composes",
    "conflicts_with",
    "depends_on",
    "extends",
    "implements",
    "similar_to",
    "supersedes",
    "uses",
];

use super::llm_extraction::{ExtractionResult, ExtractedConcept, ExtractedDecision, ExtractedRelation};

/// Parse and validate an LLM extraction response.
///
/// Returns valid entities as `Vec<GraphEntity>`, skipping malformed entries.
///
/// `turn_index` identifies the turn that triggered extraction so that
/// `Decides` edges point from the correct Turn node (not the Session),
/// matching the `edge Decides: Turn -> Decision` schema definition.
pub fn parse_and_validate_response(
    json_str: &str,
    session_slug: &str,
    turn_index: u32,
) -> Result<Vec<GraphEntity>, String> {
    let result: ExtractionResult = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse LLM extraction response: {e}"))?;

    let mut entities = Vec::new();

    for concept in &result.concepts {
        match validate_concept(concept) {
            Ok(entity) => entities.push(entity),
            Err(reason) => {
                warn!(
                    slug = concept.slug.as_deref().unwrap_or("<missing>"),
                    reason, "Skipping invalid concept"
                );
            }
        }
    }

    for decision in &result.decisions {
        match validate_decision(decision, session_slug, turn_index) {
            Ok(mut new_entities) => entities.append(&mut new_entities),
            Err(reason) => {
                warn!(
                    slug = decision.slug.as_deref().unwrap_or("<missing>"),
                    reason, "Skipping invalid decision"
                );
            }
        }
    }

    for relation in &result.relations {
        match validate_relation(relation) {
            Ok(entity) => entities.push(entity),
            Err(reason) => {
                warn!(
                    from = relation.from.as_deref().unwrap_or("<missing>"),
                    to = relation.to.as_deref().unwrap_or("<missing>"),
                    reason,
                    "Skipping invalid relation"
                );
            }
        }
    }

    Ok(entities)
}

/// Validate a single extracted concept.
fn validate_concept(concept: &ExtractedConcept) -> Result<GraphEntity, String> {
    let slug = concept
        .slug
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty slug")?;

    let name = concept
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty name")?;

    let category = concept
        .category
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty category")?;

    if !VALID_CONCEPT_CATEGORIES.contains(&category) {
        return Err(format!("Invalid category: '{category}'"));
    }

    let confidence = concept.confidence.as_deref().unwrap_or("medium");
    if !["high", "medium", "low"].contains(&confidence) {
        return Err(format!("Invalid confidence: '{confidence}'"));
    }

    let mut props = Map::new();
    props.insert("slug".to_string(), serde_json::Value::String(slug.to_string()));
    props.insert("name".to_string(), serde_json::Value::String(name.to_string()));
    props.insert("category".to_string(), serde_json::Value::String(category.to_string()));
    props.insert("summary".to_string(), serde_json::Value::String(
        concept.summary.as_deref().unwrap_or("").to_string()
    ));
    props.insert("confidence".to_string(), serde_json::Value::String(confidence.to_string()));
    props.insert("mentionCount".to_string(), serde_json::Value::Number(1.into()));

    // Required DateTime fields — default to current time
    let now = chrono::Utc::now().to_rfc3339();
    props.insert("firstSeen".to_string(), serde_json::Value::String(now.clone()));
    props.insert("lastSeen".to_string(), serde_json::Value::String(now));

    Ok(GraphEntity::Node {
        node_type: "Concept".to_string(),
        slug: slug.to_string(),
        properties: props,
    })
}

/// Validate a single extracted decision.
///
/// Produces a Turn node (so the Decides edge `from` resolves), a Decision
/// node, and a `Decides` edge from Turn → Decision — matching the schema
/// `edge Decides: Turn -> Decision`.
fn validate_decision(
    decision: &ExtractedDecision,
    session_slug: &str,
    turn_index: u32,
) -> Result<Vec<GraphEntity>, String> {
    let slug = decision
        .slug
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty slug")?;

    let title = decision
        .title
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty title")?;

    let domain = decision
        .domain
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty domain")?;

    if !VALID_DECISION_DOMAINS.contains(&domain) {
        return Err(format!("Invalid domain: '{domain}'"));
    }

    let confidence = decision.confidence.as_deref().unwrap_or("medium");
    if !["high", "medium", "low"].contains(&confidence) {
        return Err(format!("Invalid confidence: '{confidence}'"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let t_slug = turn_slug(session_slug, turn_index);

    // Turn node — created inline so the Decides edge can resolve.
    // Duplicate turns (same slug) are safely merged via @key upsert.
    let mut turn_props = Map::new();
    turn_props.insert("slug".to_string(), serde_json::Value::String(t_slug.clone()));
    turn_props.insert("sessionSlug".to_string(), serde_json::Value::String(session_slug.to_string()));
    turn_props.insert("turnIndex".to_string(), serde_json::Value::Number(turn_index.into()));
    turn_props.insert("role".to_string(), serde_json::Value::String("assistant".to_string()));
    turn_props.insert("timestamp".to_string(), serde_json::Value::String(now.clone()));

    let mut props = Map::new();
    props.insert("slug".to_string(), serde_json::Value::String(slug.to_string()));
    props.insert("title".to_string(), serde_json::Value::String(title.to_string()));
    if let Some(rationale) = decision.rationale.as_deref() {
        props.insert("rationale".to_string(), serde_json::Value::String(rationale.to_string()));
    }
    props.insert("domain".to_string(), serde_json::Value::String(domain.to_string()));
    props.insert("status".to_string(), serde_json::Value::String("active".to_string()));
    props.insert("confidence".to_string(), serde_json::Value::String(confidence.to_string()));

    // Required DateTime fields — default to current time
    props.insert("decidedAt".to_string(), serde_json::Value::String(now.clone()));
    props.insert("createdAt".to_string(), serde_json::Value::String(now.clone()));

    let mut entities = vec![
        // Turn node first
        GraphEntity::Node {
            node_type: "Turn".to_string(),
            slug: t_slug.clone(),
            properties: turn_props,
        },
        // Decision node
        GraphEntity::Node {
            node_type: "Decision".to_string(),
            slug: slug.to_string(),
            properties: props,
        },
    ];

    // Decides edge: Turn → Decision (matches schema: edge Decides: Turn -> Decision)
    let mut edge_props = Map::new();
    edge_props.insert("extractedAt".to_string(), serde_json::Value::String(now));
    entities.push(GraphEntity::Edge {
        edge_type: "Decides".to_string(),
        from_slug: t_slug,
        to_slug: slug.to_string(),
        properties: edge_props,
    });

    Ok(entities)
}

/// Validate a single extracted relation.
fn validate_relation(relation: &ExtractedRelation) -> Result<GraphEntity, String> {
    let from = relation
        .from
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty 'from'")?;

    let to = relation
        .to
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty 'to'")?;

    if from == to {
        return Err(format!("Self-referencing relation: '{from}' → '{to}'"));
    }

    let relation_type = relation
        .relation_type
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or("Missing or empty relation type")?;

    if !VALID_RELATION_TYPES.contains(&relation_type) {
        return Err(format!("Invalid relation type: '{relation_type}'"));
    }

    let strength = relation.strength.unwrap_or(0.5);
    if !(0.0..=1.0).contains(&strength) {
        return Err(format!("Strength out of range: {strength}"));
    }

    let mut props = Map::new();
    props.insert(
        "relationType".to_string(),
        serde_json::Value::String(relation_type.to_string()),
    );
    props.insert(
        "strength".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(strength)
                .ok_or_else(|| format!("Non-finite strength value: {strength}"))?
        ),
    );
    props.insert(
        "coOccurrenceCount".to_string(),
        serde_json::Value::Number(1.into()),
    );

    // Required DateTime fields for RelatesTo edges
    let now = chrono::Utc::now().to_rfc3339();
    props.insert("firstSeen".to_string(), serde_json::Value::String(now.clone()));
    props.insert("lastSeen".to_string(), serde_json::Value::String(now));

    Ok(GraphEntity::Edge {
        edge_type: "RelatesTo".to_string(),
        from_slug: from.to_string(),
        to_slug: to.to_string(),
        properties: props,
    })
}
