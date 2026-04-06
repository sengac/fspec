//! Shared test helpers for graph entity tests.
//!
//! Provides utility functions for writing test files, counting entities,
//! finding nodes/edges, and building common graph entity types.
//!
//! Note: Each test binary includes this module via `mod graph_test_helpers;`
//! but only uses a subset of helpers, so dead_code is expected.

#![allow(dead_code)]

use codelet_napi::graph::graph_entities::GraphEntity;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

/// Write a file and return its path.
pub fn write_test_file(dir: &Path, rel_path: &str, content: &str) -> std::path::PathBuf {
    let full_path = dir.join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create dir");
    }
    std::fs::write(&full_path, content).expect("write file");
    full_path
}

/// Count entities by node type.
pub fn count_nodes(entities: &[GraphEntity], node_type: &str) -> usize {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type: nt, .. } if nt == node_type))
        .count()
}

/// Count entities by edge type.
pub fn count_edges(entities: &[GraphEntity], edge_type: &str) -> usize {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type: et, .. } if et == edge_type))
        .count()
}

/// Find a node by type and slug.
pub fn find_node<'a>(
    entities: &'a [GraphEntity],
    node_type: &str,
    slug: &str,
) -> Option<&'a GraphEntity> {
    entities.iter().find(|e| {
        matches!(e, GraphEntity::Node { node_type: nt, slug: s, .. } if nt == node_type && s == slug)
    })
}

/// Get a string property from a node entity.
pub fn get_node_property<'a>(entity: &'a GraphEntity, key: &str) -> Option<&'a str> {
    if let GraphEntity::Node { properties, .. } = entity {
        properties.get(key).and_then(|v| v.as_str())
    } else {
        None
    }
}

/// Find any Dependency node and verify it has the expected source.
pub fn has_dependency_with_source(entities: &[GraphEntity], expected_source: &str) -> bool {
    entities.iter().any(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "Dependency"
                && (properties
                    .get("source")
                    .and_then(|v| v.as_str()) == Some(expected_source))
        } else {
            false
        }
    })
}

// ============================================================================
// Edge Extraction Test Helpers
// ============================================================================

/// Find edges by type with optional from/to slug substring match.
///
/// Used by all edge extraction integration tests.
pub fn find_edges<'a>(
    entities: &'a [GraphEntity],
    edge_type: &str,
    from_contains: Option<&str>,
    to_contains: Option<&str>,
) -> Vec<&'a GraphEntity> {
    entities
        .iter()
        .filter(|e| match e {
            GraphEntity::Edge {
                edge_type: et,
                from_slug,
                to_slug,
                ..
            } => {
                et == edge_type
                    && from_contains.is_none_or(|f| from_slug.contains(f))
                    && to_contains.is_none_or(|t| to_slug.contains(t))
            }
            _ => false,
        })
        .collect()
}

/// Build known_files set from a temp directory for import resolution.
///
/// Recursively walks the directory and collects relative file paths.
pub fn build_known_files(dir: &std::path::Path) -> HashSet<String> {
    let mut known = HashSet::new();
    collect_files_recursive(dir, dir, &mut known);
    known
}

/// Recursively collect file paths relative to root.
fn collect_files_recursive(
    current: &std::path::Path,
    root: &std::path::Path,
    known: &mut HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, root, known);
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    known.insert(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
}

// ============================================================================
// Learnings Entity Builders
// ============================================================================

/// Build a Learning node entity for test purposes.
pub fn make_learning(slug: &str, title: &str, category: &str, content: &str) -> GraphEntity {
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(slug.to_string()));
    props.insert("title".to_string(), Value::String(title.to_string()));
    props.insert("category".to_string(), Value::String(category.to_string()));
    props.insert("content".to_string(), Value::String(content.to_string()));
    props.insert("confidence".to_string(), Value::String("high".to_string()));
    props.insert(
        "firstSeen".to_string(),
        Value::String("2026-01-01T00:00:00Z".to_string()),
    );
    props.insert(
        "lastSeen".to_string(),
        Value::String("2026-01-01T00:00:00Z".to_string()),
    );
    props.insert("mentionCount".to_string(), Value::Number(1.into()));
    GraphEntity::Node {
        node_type: "Learning".to_string(),
        slug: slug.to_string(),
        properties: props,
    }
}

/// Build a Decision node entity for test purposes.
pub fn make_decision(
    slug: &str,
    title: &str,
    domain: &str,
    status: &str,
    rationale: &str,
) -> GraphEntity {
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(slug.to_string()));
    props.insert("title".to_string(), Value::String(title.to_string()));
    props.insert("domain".to_string(), Value::String(domain.to_string()));
    props.insert("status".to_string(), Value::String(status.to_string()));
    props.insert(
        "rationale".to_string(),
        Value::String(rationale.to_string()),
    );
    props.insert(
        "decidedAt".to_string(),
        Value::String("2026-01-01T00:00:00Z".to_string()),
    );
    props.insert(
        "createdAt".to_string(),
        Value::String("2026-01-01T00:00:00Z".to_string()),
    );
    GraphEntity::Node {
        node_type: "Decision".to_string(),
        slug: slug.to_string(),
        properties: props,
    }
}

/// Build an Exploration node entity for test purposes.
pub fn make_exploration(
    slug: &str,
    title: &str,
    outcome: &str,
    failure_constraint: Option<&str>,
) -> GraphEntity {
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(slug.to_string()));
    props.insert("title".to_string(), Value::String(title.to_string()));
    props.insert(
        "strategy".to_string(),
        Value::String("test strategy".to_string()),
    );
    props.insert("outcome".to_string(), Value::String(outcome.to_string()));
    props.insert(
        "createdAt".to_string(),
        Value::String("2026-01-01T00:00:00Z".to_string()),
    );
    if let Some(fc) = failure_constraint {
        props.insert(
            "failureConstraint".to_string(),
            Value::String(fc.to_string()),
        );
    }
    GraphEntity::Node {
        node_type: "Exploration".to_string(),
        slug: slug.to_string(),
        properties: props,
    }
}
