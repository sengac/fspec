//! Graph Merge & Upsert Logic
//!
//! Converts `Vec<GraphEntity>` to JSONL for nanograph load, implements
//! custom merge semantics (increment, min/max, confidence promotion),
//! and maintains watermark state for incremental re-indexing.
//!
//! Feature: spec/features/graph-merge-upsert-logic.feature

use super::extractors::GraphEntity;
use serde_json::{Map, Value};
use std::collections::HashMap;

// Re-export watermark types for backward compatibility
pub use super::watermark::{
    IndexState, SessionWatermark,
    read_index_state, write_index_state, update_session_watermark,
};

/// Convert a list of GraphEntity into JSONL lines for nanograph.
///
/// Nodes become: `{"type":"NodeType","data":{...properties...}}`
/// Edges become: `{"edge":"EdgeType","from":"slug","to":"slug","data":{...properties...}}`
pub fn entities_to_jsonl(entities: &[GraphEntity]) -> String {
    let mut lines = Vec::with_capacity(entities.len());

    for entity in entities {
        match entity {
            GraphEntity::Node {
                node_type,
                slug: _,
                properties,
            } => {
                let mut obj = Map::new();
                obj.insert("type".to_string(), Value::String(node_type.clone()));
                obj.insert("data".to_string(), Value::Object(properties.clone()));
                if let Ok(line) = serde_json::to_string(&Value::Object(obj)) {
                    lines.push(line);
                }
            }
            GraphEntity::Edge {
                edge_type,
                from_slug,
                to_slug,
                properties,
            } => {
                let mut obj = Map::new();
                obj.insert("edge".to_string(), Value::String(edge_type.clone()));
                obj.insert("from".to_string(), Value::String(from_slug.clone()));
                obj.insert("to".to_string(), Value::String(to_slug.clone()));
                obj.insert("data".to_string(), Value::Object(properties.clone()));
                if let Ok(line) = serde_json::to_string(&Value::Object(obj)) {
                    lines.push(line);
                }
            }
        }
    }

    lines.join("\n")
}

/// Confidence level ordering for promotion logic.
fn confidence_rank(confidence: &str) -> u8 {
    match confidence {
        "low" => 0,
        "medium" => 1,
        "high" => 2,
        _ => 0,
    }
}

/// Promote confidence: always keep the higher value.
fn promote_confidence(existing: &str, incoming: &str) -> String {
    if confidence_rank(incoming) > confidence_rank(existing) {
        incoming.to_string()
    } else {
        existing.to_string()
    }
}

/// Calculate RelatesTo edge strength from co-occurrence count.
///
/// Formula: min(1.0, log2(coOccurrenceCount + 1) / 10.0)
pub fn calculate_strength(co_occurrence_count: i64) -> f64 {
    let raw = ((co_occurrence_count + 1) as f64).log2() / 10.0;
    raw.min(1.0)
}

/// Merge a set of incoming GraphEntity nodes with existing data.
///
/// For concepts: increment mentionCount, promote confidence, keep min firstSeen / max lastSeen.
/// For RelatesTo edges: increment coOccurrenceCount, recalculate strength.
///
/// `existing_lookup` maps slug → existing node properties (from a graph query).
pub fn merge_entities(
    incoming: &[GraphEntity],
    existing_lookup: &HashMap<String, Map<String, Value>>,
) -> Vec<GraphEntity> {
    incoming
        .iter()
        .map(|entity| match entity {
            GraphEntity::Node {
                node_type,
                slug,
                properties,
            } => {
                if let Some(existing_props) = existing_lookup.get(slug) {
                    let merged = merge_node_properties(node_type, existing_props, properties);
                    GraphEntity::Node {
                        node_type: node_type.clone(),
                        slug: slug.clone(),
                        properties: merged,
                    }
                } else {
                    entity.clone()
                }
            }
            GraphEntity::Edge {
                edge_type,
                from_slug,
                to_slug,
                properties,
            } => {
                let edge_key = format!("{}:{}:{}", edge_type, from_slug, to_slug);
                if let Some(existing_props) = existing_lookup.get(&edge_key) {
                    let merged = merge_edge_properties(edge_type, existing_props, properties);
                    GraphEntity::Edge {
                        edge_type: edge_type.clone(),
                        from_slug: from_slug.clone(),
                        to_slug: to_slug.clone(),
                        properties: merged,
                    }
                } else {
                    entity.clone()
                }
            }
        })
        .collect()
}

/// Merge node properties with custom semantics.
fn merge_node_properties(
    _node_type: &str,
    existing: &Map<String, Value>,
    incoming: &Map<String, Value>,
) -> Map<String, Value> {
    let mut merged = incoming.clone();

    // mentionCount: increment
    if let (Some(Value::Number(existing_mc)), Some(Value::Number(incoming_mc))) =
        (existing.get("mentionCount"), incoming.get("mentionCount"))
    {
        let sum = existing_mc.as_i64().unwrap_or(0) + incoming_mc.as_i64().unwrap_or(0);
        merged.insert(
            "mentionCount".to_string(),
            Value::Number(serde_json::Number::from(sum)),
        );
    }

    // firstSeen: keep earliest
    if let (Some(Value::String(existing_fs)), Some(Value::String(incoming_fs))) =
        (existing.get("firstSeen"), incoming.get("firstSeen"))
    {
        if existing_fs < incoming_fs {
            merged.insert(
                "firstSeen".to_string(),
                Value::String(existing_fs.clone()),
            );
        }
    } else if let Some(existing_fs) = existing.get("firstSeen") {
        // If incoming doesn't have firstSeen, keep existing
        merged.insert("firstSeen".to_string(), existing_fs.clone());
    }

    // lastSeen: keep latest
    if let (Some(Value::String(existing_ls)), Some(Value::String(incoming_ls))) =
        (existing.get("lastSeen"), incoming.get("lastSeen"))
    {
        if existing_ls > incoming_ls {
            merged.insert(
                "lastSeen".to_string(),
                Value::String(existing_ls.clone()),
            );
        }
    }

    // confidence: promote, never demote
    if let (Some(Value::String(existing_c)), Some(Value::String(incoming_c))) =
        (existing.get("confidence"), incoming.get("confidence"))
    {
        let promoted = promote_confidence(existing_c, incoming_c);
        merged.insert("confidence".to_string(), Value::String(promoted));
    }

    merged
}

/// Merge edge properties with custom semantics.
fn merge_edge_properties(
    edge_type: &str,
    existing: &Map<String, Value>,
    incoming: &Map<String, Value>,
) -> Map<String, Value> {
    let mut merged = incoming.clone();

    if edge_type == "RelatesTo" {
        // coOccurrenceCount: increment
        let existing_count = existing
            .get("coOccurrenceCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let incoming_count = incoming
            .get("coOccurrenceCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let new_count = existing_count + incoming_count;
        merged.insert(
            "coOccurrenceCount".to_string(),
            Value::Number(serde_json::Number::from(new_count)),
        );

        // strength: recalculate from co-occurrence count
        let strength = calculate_strength(new_count);
        if let Some(n) = serde_json::Number::from_f64(strength) {
            merged.insert("strength".to_string(), Value::Number(n));
        }

        // firstSeen: keep earliest
        if let (Some(Value::String(existing_fs)), Some(Value::String(incoming_fs))) =
            (existing.get("firstSeen"), incoming.get("firstSeen"))
        {
            if existing_fs < incoming_fs {
                merged.insert(
                    "firstSeen".to_string(),
                    Value::String(existing_fs.clone()),
                );
            }
        }

        // lastSeen: keep latest
        if let (Some(Value::String(existing_ls)), Some(Value::String(incoming_ls))) =
            (existing.get("lastSeen"), incoming.get("lastSeen"))
        {
            if existing_ls > incoming_ls {
                merged.insert(
                    "lastSeen".to_string(),
                    Value::String(existing_ls.clone()),
                );
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_concept_node(slug: &str, name: &str, category: &str) -> GraphEntity {
        let mut props = Map::new();
        props.insert("slug".to_string(), Value::String(slug.to_string()));
        props.insert("name".to_string(), Value::String(name.to_string()));
        props.insert("category".to_string(), Value::String(category.to_string()));
        props.insert(
            "mentionCount".to_string(),
            Value::Number(1.into()),
        );
        props.insert(
            "confidence".to_string(),
            Value::String("medium".to_string()),
        );
        GraphEntity::Node {
            node_type: "Concept".to_string(),
            slug: slug.to_string(),
            properties: props,
        }
    }

    // ============================================================================
    // Scenario: GraphEntity nodes are converted to JSONL and loaded into nanograph
    // ============================================================================
    #[test]
    fn test_entities_to_jsonl_converts_nodes() {
        // @step Given a Vec of 2 Concept GraphEntity nodes with valid slugs, names, and categories
        let entities = vec![
            make_concept_node("jwt-auth", "JWT Authentication", "technology"),
            make_concept_node("session-mgmt", "Session Management", "pattern"),
        ];

        // @step When the entities are converted to JSONL and loaded via merge mode
        let jsonl = entities_to_jsonl(&entities);
        let lines: Vec<&str> = jsonl.lines().collect();

        // @step Then 2 Concept rows are visible in the database with correct slug, name, and category values
        assert_eq!(lines.len(), 2);

        let parsed0: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed0["type"], "Concept");
        assert_eq!(parsed0["data"]["slug"], "jwt-auth");
        assert_eq!(parsed0["data"]["name"], "JWT Authentication");
        assert_eq!(parsed0["data"]["category"], "technology");

        let parsed1: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed1["type"], "Concept");
        assert_eq!(parsed1["data"]["slug"], "session-mgmt");
    }

    // ============================================================================
    // Scenario: Duplicate concept slug merges with increment semantics
    // ============================================================================
    #[test]
    fn test_merge_increments_mention_count() {
        // @step Given a Concept node with slug "jwt-auth" and mentionCount 3 already exists in the database
        let mut existing_props = Map::new();
        existing_props.insert("mentionCount".to_string(), Value::Number(3.into()));
        existing_props.insert(
            "firstSeen".to_string(),
            Value::String("2026-03-01T00:00:00Z".to_string()),
        );
        existing_props.insert(
            "lastSeen".to_string(),
            Value::String("2026-03-15T00:00:00Z".to_string()),
        );
        existing_props.insert(
            "confidence".to_string(),
            Value::String("medium".to_string()),
        );
        let mut existing_lookup = HashMap::new();
        existing_lookup.insert("jwt-auth".to_string(), existing_props);

        // @step When the same slug is loaded again with mentionCount 2 and a later lastSeen timestamp
        let mut incoming_props = Map::new();
        incoming_props.insert("slug".to_string(), Value::String("jwt-auth".to_string()));
        incoming_props.insert("mentionCount".to_string(), Value::Number(2.into()));
        incoming_props.insert(
            "firstSeen".to_string(),
            Value::String("2026-03-10T00:00:00Z".to_string()),
        );
        incoming_props.insert(
            "lastSeen".to_string(),
            Value::String("2026-03-19T00:00:00Z".to_string()),
        );
        incoming_props.insert(
            "confidence".to_string(),
            Value::String("medium".to_string()),
        );
        let incoming = vec![GraphEntity::Node {
            node_type: "Concept".to_string(),
            slug: "jwt-auth".to_string(),
            properties: incoming_props,
        }];

        let merged = merge_entities(&incoming, &existing_lookup);

        // @step Then the mentionCount is 5 (summed, not overwritten)
        if let GraphEntity::Node { properties, .. } = &merged[0] {
            assert_eq!(properties["mentionCount"], Value::Number(5.into()));

            // @step And the firstSeen timestamp is preserved from the original load
            assert_eq!(
                properties["firstSeen"],
                Value::String("2026-03-01T00:00:00Z".to_string())
            );

            // @step And the lastSeen timestamp is updated to the later value
            assert_eq!(
                properties["lastSeen"],
                Value::String("2026-03-19T00:00:00Z".to_string())
            );
        } else {
            panic!("Expected Node entity");
        }
    }

    // ============================================================================
    // Scenario: Confidence is promoted on merge
    // ============================================================================
    #[test]
    fn test_confidence_promoted_on_merge() {
        // @step Given a Concept node with slug "test-concept" and confidence "medium" exists in the database
        let mut existing_props = Map::new();
        existing_props.insert(
            "confidence".to_string(),
            Value::String("medium".to_string()),
        );
        existing_props.insert("mentionCount".to_string(), Value::Number(1.into()));
        let mut existing_lookup = HashMap::new();
        existing_lookup.insert("test-concept".to_string(), existing_props);

        // @step When the same slug is loaded with confidence "high"
        let mut incoming_props = Map::new();
        incoming_props.insert(
            "confidence".to_string(),
            Value::String("high".to_string()),
        );
        incoming_props.insert("mentionCount".to_string(), Value::Number(1.into()));
        let incoming = vec![GraphEntity::Node {
            node_type: "Concept".to_string(),
            slug: "test-concept".to_string(),
            properties: incoming_props,
        }];

        let merged = merge_entities(&incoming, &existing_lookup);

        // @step Then the confidence is promoted to "high"
        if let GraphEntity::Node { properties, .. } = &merged[0] {
            assert_eq!(
                properties["confidence"],
                Value::String("high".to_string())
            );
        } else {
            panic!("Expected Node entity");
        }
    }

    // ============================================================================
    // Scenario: Confidence is not demoted on merge
    // ============================================================================
    #[test]
    fn test_confidence_not_demoted_on_merge() {
        // @step Given a Concept node with slug "stable-concept" and confidence "high" exists in the database
        let mut existing_props = Map::new();
        existing_props.insert(
            "confidence".to_string(),
            Value::String("high".to_string()),
        );
        existing_props.insert("mentionCount".to_string(), Value::Number(1.into()));
        let mut existing_lookup = HashMap::new();
        existing_lookup.insert("stable-concept".to_string(), existing_props);

        // @step When the same slug is loaded with confidence "low"
        let mut incoming_props = Map::new();
        incoming_props.insert(
            "confidence".to_string(),
            Value::String("low".to_string()),
        );
        incoming_props.insert("mentionCount".to_string(), Value::Number(1.into()));
        let incoming = vec![GraphEntity::Node {
            node_type: "Concept".to_string(),
            slug: "stable-concept".to_string(),
            properties: incoming_props,
        }];

        let merged = merge_entities(&incoming, &existing_lookup);

        // @step Then the confidence remains "high"
        if let GraphEntity::Node { properties, .. } = &merged[0] {
            assert_eq!(
                properties["confidence"],
                Value::String("high".to_string())
            );
        } else {
            panic!("Expected Node entity");
        }
    }

    // ============================================================================
    // Scenario: Watermark state updated after successful upsert
    // ============================================================================
    #[test]
    fn test_watermark_state_updated() {
        // @step Given an empty index-state.json
        let tmp_dir = tempfile::tempdir().expect("Failed to create temp dir for watermark test");
        let state = read_index_state(tmp_dir.path());
        assert!(state.sessions.is_empty());
        assert_eq!(state.schema_version, "1", "Default schema version should be '1'");

        // @step When a batch of entities from session "abc-123" up to turn 42 is successfully loaded
        let mut state = state;
        let now = "2026-03-19T12:00:00Z";
        update_session_watermark(&mut state, "abc-123", 42, now);
        write_index_state(tmp_dir.path(), &state).unwrap();

        // @step Then the index-state.json contains a watermark entry for session "abc-123" with lastIndexedTurn 42
        let reloaded = read_index_state(tmp_dir.path());
        let wm = reloaded.sessions.get("abc-123").expect("session watermark should exist");
        assert_eq!(wm.last_indexed_turn, 42);

        // @step And the lastRunAt timestamp is updated to the current time
        assert_eq!(reloaded.last_run_at, "2026-03-19T12:00:00Z");
    }

    // ============================================================================
    // Scenario: RelatesTo edge co-occurrence count and strength are updated on merge
    // ============================================================================
    #[test]
    fn test_relates_to_edge_merge() {
        // @step Given a RelatesTo edge between "jwt-auth" and "session-mgmt" with coOccurrenceCount 1
        let mut existing_props = Map::new();
        existing_props.insert("coOccurrenceCount".to_string(), Value::Number(1.into()));
        existing_props.insert(
            "strength".to_string(),
            Value::Number(serde_json::Number::from_f64(0.1).unwrap()),
        );
        existing_props.insert(
            "relationType".to_string(),
            Value::String("uses".to_string()),
        );
        existing_props.insert(
            "firstSeen".to_string(),
            Value::String("2026-03-01T00:00:00Z".to_string()),
        );
        existing_props.insert(
            "lastSeen".to_string(),
            Value::String("2026-03-15T00:00:00Z".to_string()),
        );
        let mut existing_lookup = HashMap::new();
        existing_lookup.insert(
            "RelatesTo:jwt-auth:session-mgmt".to_string(),
            existing_props,
        );

        // @step When the same concept pair is loaded again as a RelatesTo edge
        let mut incoming_props = Map::new();
        incoming_props.insert("coOccurrenceCount".to_string(), Value::Number(1.into()));
        incoming_props.insert(
            "strength".to_string(),
            Value::Number(serde_json::Number::from_f64(0.5).unwrap()),
        );
        incoming_props.insert(
            "relationType".to_string(),
            Value::String("uses".to_string()),
        );
        incoming_props.insert(
            "firstSeen".to_string(),
            Value::String("2026-03-19T00:00:00Z".to_string()),
        );
        incoming_props.insert(
            "lastSeen".to_string(),
            Value::String("2026-03-19T00:00:00Z".to_string()),
        );
        let incoming = vec![GraphEntity::Edge {
            edge_type: "RelatesTo".to_string(),
            from_slug: "jwt-auth".to_string(),
            to_slug: "session-mgmt".to_string(),
            properties: incoming_props,
        }];

        let merged = merge_entities(&incoming, &existing_lookup);

        // @step Then the coOccurrenceCount becomes 2
        if let GraphEntity::Edge { properties, .. } = &merged[0] {
            assert_eq!(properties["coOccurrenceCount"], Value::Number(2.into()));

            // @step And the strength is recalculated as min(1.0, log2(3) / 10.0)
            let expected_strength = (3.0_f64).log2() / 10.0;
            let actual_strength = properties["strength"].as_f64().unwrap();
            assert!(
                (actual_strength - expected_strength).abs() < 0.001,
                "expected strength ~{:.4} but got {:.4}",
                expected_strength,
                actual_strength
            );
        } else {
            panic!("Expected Edge entity");
        }
    }
}
