//! Graph Entity Types — Shared Data Types for Graph Pipelines
//!
//! Core types used by both the AST extraction pipeline and the Learnings
//! extraction pipeline. Extracted as a shared module to avoid duplication.

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
