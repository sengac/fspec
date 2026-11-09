// Feature: spec/features/ast-index-class-import-crash.feature
//
// Tests for KGRAPH-055: Python and Java ast_index crashes on real repos.
// Root causes:
// 1. resolve_calls first_char.is_uppercase() heuristic fails for underscore-prefixed class names
// 2. extract_name_after_keyword picks up keyword from comments before actual declaration
// 3. deduplicate_entities edge pruning doesn't check node type matches edge schema
//
// The fact engine (KGRAPH-074/075) resolves every reference exactly: a
// call naming a class is its constructor and never a Calls edge, and no
// edge is ever built to a node of the wrong type.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::{named_edges, write_test_file};

/// Find nodes by type and name substring.
fn find_nodes_by_name<'a>(
    entities: &'a [GraphEntity],
    node_type: &str,
    name_contains: &str,
) -> Vec<&'a GraphEntity> {
    entities
        .iter()
        .filter(|e| match e {
            GraphEntity::Node {
                node_type: nt,
                properties,
                ..
            } => {
                nt == node_type
                    && properties
                        .get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|n| n.contains(name_contains))
            }
            _ => false,
        })
        .collect()
}

/// Index `root` with the real pipeline (one thread, no gitignore).
fn index(root: &std::path::Path) -> Indexed {
    index_tree(root, false, 1)
}

/// The count of unresolved references of `kind` with `reason`.
fn unresolved(indexed: &Indexed, kind: &str, reason: &str) -> u64 {
    let counts = serde_json::to_value(&indexed.unresolved).expect("serialize unresolved");
    counts[kind][reason].as_u64().unwrap_or(0)
}

/// Every Calls edge whose target is a Type node.
fn calls_to_types(entities: &[GraphEntity]) -> Vec<&GraphEntity> {
    let types: std::collections::BTreeSet<&str> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "Type" => Some(slug.as_str()),
            _ => None,
        })
        .collect();
    entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Edge { edge_type, to_slug, .. }
                if edge_type == "Calls" && types.contains(to_slug.as_str()))
        })
        .collect()
}

/// The (from, to) qualifiedName pairs of `edge_type` edges whose from or
/// to name contains `needle`.
fn edges_touching(
    entities: &[GraphEntity],
    edge_type: &str,
    needle: &str,
) -> Vec<(String, String)> {
    named_edges(entities, edge_type)
        .into_iter()
        .filter(|(from, to)| from.contains(needle) || to.contains(needle))
        .collect()
}

// ============================================================================
// Scenario: Python underscore-prefixed class imported and called as constructor
//           produces TypeRef
// ============================================================================

#[path = "ast_class_import_crash_test/deduplicate_prunes_calls_edge.rs"]
mod deduplicate_prunes_calls_edge;
#[path = "ast_class_import_crash_test/python_underscore_class_import.rs"]
mod python_underscore_class_import;

// ============================================================================
// Scenario: Java class with comment containing keyword before declaration
//           extracts correct name
// ============================================================================

// ============================================================================
// Scenario: Java imported class used in constructor produces TypeRef not crash
// ============================================================================

// ============================================================================
// Scenario: Python mixed imports — function gets Calls edge, class gets TypeRef edge
// ============================================================================

// ============================================================================
// Scenario: Deduplicate prunes Calls edge targeting a Type-only slug
// ============================================================================

// ============================================================================
// Scenario: Python stdlib import does not create edges or crash
// ============================================================================
