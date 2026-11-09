// Feature: spec/features/dart-ast-index-support.feature
//
// Integration tests for Dart AST extraction: File, Function, Type nodes,
// plus Imports, Calls, and TypeRef edges, from indexed temp projects (fact
// engine + resolve pass). Also tests pubspec.yaml dependency extraction and
// integration with walk_and_extract dispatch.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::index_tree;
use codelet_graph::ast_pipeline::pubspec_dep_extractor::extract_pubspec_dependencies;
use codelet_graph::graph_entities::GraphEntity;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{edges, nodes, project};

/// A temp project holding `files`, indexed with the fact engine and the
/// resolve pass; returns the temp directory (keep it alive), the root and
/// the entities.
fn indexed(
    files: &[(&str, &str)],
) -> Result<(tempfile::TempDir, std::path::PathBuf, Vec<GraphEntity>), TestError> {
    let (directory, root) = project(files)?;
    let indexed = index_tree(&root, true, 1);
    assert!(indexed.failed.is_empty(), "failed: {:?}", indexed.failed);
    Ok((directory, root, indexed.entities))
}

/// Count entities by node type.
fn count_nodes(entities: &[GraphEntity], node_type: &str) -> usize {
    nodes(entities, node_type).len()
}

/// Count entities by edge type.
fn count_edges(entities: &[GraphEntity], edge_type: &str) -> usize {
    edges(entities, edge_type).len()
}

/// The `name` of the node with this slug.
fn name_of(entities: &[GraphEntity], slug: &str) -> Option<String> {
    entities.iter().find_map(|e| match e {
        GraphEntity::Node {
            slug: s,
            properties,
            ..
        } if s == slug => properties
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    })
}

/// The `edge_type` edges as (from name, to name) — a File's name is its
/// path.
fn named_edges(entities: &[GraphEntity], edge_type: &str) -> Vec<(String, String)> {
    edges(entities, edge_type)
        .into_iter()
        .map(|(from, to, _)| {
            (
                name_of(entities, &from).unwrap_or(from),
                name_of(entities, &to).unwrap_or(to),
            )
        })
        .collect()
}

/// Whether an `edge_type` edge leads from a node named `from` to a node
/// named `to`.
fn has_edge(entities: &[GraphEntity], edge_type: &str, from: &str, to: &str) -> bool {
    named_edges(entities, edge_type)
        .iter()
        .any(|(f, t)| f == from && t == to)
}

/// The names of the `node_type` nodes, sorted.
fn names(entities: &[GraphEntity], node_type: &str) -> Vec<String> {
    let mut found: Vec<String> = nodes(entities, node_type)
        .into_iter()
        .filter_map(|(_, p)| p.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();
    found.sort();
    found
}

#[path = "dart_edge_extraction_test/dart_dead_code_detection.rs"]
mod dart_dead_code_detection;
#[path = "dart_edge_extraction_test/dart_extract_file_function.rs"]
mod dart_extract_file_function;
#[path = "dart_edge_extraction_test/dart_extract_pubspec_dependencies.rs"]
mod dart_extract_pubspec_dependencies;
