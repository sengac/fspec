//! Graph Entity Types — Shared Data Types for Graph Pipelines
//!
//! Core types used by both the AST extraction pipeline and the Learnings
//! extraction pipeline. Extracted as a shared module to avoid duplication.
//!
//! Also provides JSONL serialization (`entities_to_jsonl`) and deserialization
//! (`jsonl_to_entities`) for portable graph bundles (KGRAPH-069).

use serde_json::{Map, Value};

/// A graph entity to be upserted into the nanograph database.
#[derive(Debug, Clone)]
pub enum GraphEntity {
    /// A node to insert/update.
    Node {
        node_type: String,
        slug: String,
        properties: Map<String, Value>,
    },
    /// An edge to insert.
    Edge {
        edge_type: String,
        from_slug: String,
        to_slug: String,
        properties: Map<String, Value>,
    },
}

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

/// Parse JSONL lines back into a list of GraphEntity.
///
/// Inverse of `entities_to_jsonl`. Recognises nodes (have `"type"` key) and
/// edges (have `"edge"` key). Blank lines are skipped.
pub fn jsonl_to_entities(jsonl: &str) -> Result<Vec<GraphEntity>, String> {
    let mut entities = Vec::new();

    for (i, line) in jsonl.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let obj: Map<String, Value> = serde_json::from_str(trimmed)
            .map_err(|e| format!("line {}: invalid JSON: {e}", i + 1))?;

        if let Some(Value::String(node_type)) = obj.get("type") {
            // Node line
            let properties = match obj.get("data") {
                Some(Value::Object(data)) => data.clone(),
                _ => Map::new(),
            };
            let slug = properties
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            entities.push(GraphEntity::Node {
                node_type: node_type.clone(),
                slug,
                properties,
            });
        } else if let Some(Value::String(edge_type)) = obj.get("edge") {
            // Edge line
            let from_slug = obj
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let to_slug = obj
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let properties = match obj.get("data") {
                Some(Value::Object(data)) => data.clone(),
                _ => Map::new(),
            };
            entities.push(GraphEntity::Edge {
                edge_type: edge_type.clone(),
                from_slug,
                to_slug,
                properties,
            });
        } else {
            return Err(format!(
                "line {}: unrecognised JSONL line (no 'type' or 'edge' key)",
                i + 1
            ));
        }
    }

    Ok(entities)
}
