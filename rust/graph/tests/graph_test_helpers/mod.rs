//! Shared test helpers for graph entity tests.
//!
//! Provides utility functions for writing test files, counting entities,
//! finding nodes/edges, and building common graph entity types.
//!
//! Each test binary includes this module via `pub mod graph_test_helpers;`
//! and only uses a subset of the helpers; declaring the module `pub` keeps
//! the unused `pub` helpers reachable so they do not trip `dead_code`.

use codelet_graph::graph_entities::GraphEntity;
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
                && (properties.get("source").and_then(|v| v.as_str()) == Some(expected_source))
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

/// Write `files` (relative path, text) into a fresh temp directory and
/// index it with the real pipeline (`index_tree`). Keep the returned
/// directory alive while the result is used.
pub fn index_fixture(
    files: &[(&str, &str)],
) -> (tempfile::TempDir, codelet_graph::ast_pipeline::Indexed) {
    let dir = tempfile::tempdir().expect("create temp dir");
    for (path, text) in files {
        write_test_file(dir.path(), path, text);
    }
    let indexed = codelet_graph::ast_pipeline::index_tree(dir.path(), false, 1);
    (dir, indexed)
}

/// The readable name of every node keyed by slug: a File's path, another
/// node's qualifiedName, else its slug.
pub fn node_names(entities: &[GraphEntity]) -> std::collections::BTreeMap<String, String> {
    entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type,
                slug,
                properties,
            } => {
                let key = if node_type == "File" {
                    "path"
                } else {
                    "qualifiedName"
                };
                let name = properties.get(key).and_then(|v| v.as_str()).unwrap_or(slug);
                Some((slug.clone(), name.to_string()))
            }
            _ => None,
        })
        .collect()
}

/// The (from, to) names (see `node_names`) of every `edge_type` edge, sorted.
pub fn named_edges(entities: &[GraphEntity], edge_type: &str) -> Vec<(String, String)> {
    let names = node_names(entities);
    let name = |slug: &String| names.get(slug).cloned().unwrap_or_else(|| slug.clone());
    let mut found: Vec<(String, String)> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Edge {
                edge_type: t,
                from_slug,
                to_slug,
                ..
            } if t == edge_type => Some((name(from_slug), name(to_slug))),
            _ => None,
        })
        .collect();
    found.sort();
    found
}

/// Whether an `edge_type` edge links the nodes named `from` and `to` (see
/// `node_names`).
pub fn has_named_edge(entities: &[GraphEntity], edge_type: &str, from: &str, to: &str) -> bool {
    named_edges(entities, edge_type)
        .iter()
        .any(|(f, t)| f == from && t == to)
}

/// The first node of `node_type` whose name (see `node_names`) is `name`.
pub fn node_named<'a>(
    entities: &'a [GraphEntity],
    node_type: &str,
    name: &str,
) -> Option<&'a GraphEntity> {
    entities.iter().find(|e| match e {
        GraphEntity::Node {
            node_type: t,
            properties,
            ..
        } => {
            let key = if t == "File" { "path" } else { "qualifiedName" };
            t == node_type && properties.get(key).and_then(|v| v.as_str()) == Some(name)
        }
        _ => false,
    })
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
