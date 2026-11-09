// Feature: spec/features/dart-dead-code-false-positive-reduction.feature
//
// Tests for reducing false positives in Dart/Flutter dead code detection.
// Covers: test file function exclusion, generated file exclusion, Flutter
// platform directory exclusion, main.dart entry point, extension declarations,
// qualified static calls, constructor invocations, StatefulWidget State classes,
// and verification that genuinely dead code is still reported.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use codelet_graph::ast_dead_code::dispatch_ast_dead_code;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{edges, entities, index, nodes, project};

/// The AST queries for dead code detection.
const AST_QUERIES: &str = include_str!("../schemas/ast-queries.gq");

/// Write a file under `dir`.
fn write_test_file(dir: &Path, rel_path: &str, content: &str) {
    let full_path = dir.join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create dir");
    }
    std::fs::write(&full_path, content).expect("write file");
}

/// A temp project holding `files`, indexed with `ast_index`; returns the
/// temp directory (keep it alive) and the root.
async fn indexed(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let (directory, root) = project(files)?;
    index(&root).await?;
    Ok((directory, root))
}

/// The `ast_dead_code` result for `root`'s graph.
async fn dead_code(root: &Path, entity_type: Option<&str>) -> Result<Value, TestError> {
    let graph = registry::read_code_graph(root).await?;
    let output = dispatch_ast_dead_code(&graph, entity_type, None, None).await;
    Ok(serde_json::from_str(&output)?)
}

/// The names of the functions with no incoming Calls edge (the raw
/// `uncalled_functions` query, before the dead code filters).
async fn uncalled_functions(root: &Path) -> Result<Vec<String>, TestError> {
    raw_names(root, "uncalled_functions").await
}

/// The names of the types with no incoming TypeRef edge (the raw
/// `unreferenced_types` query, before the dead code filters).
async fn unreferenced_types(root: &Path) -> Result<Vec<String>, TestError> {
    raw_names(root, "unreferenced_types").await
}

async fn raw_names(root: &Path, query: &str) -> Result<Vec<String>, TestError> {
    let graph = registry::read_code_graph(root).await?;
    let rows = graph.query_with_source(AST_QUERIES, query, None).await?;
    let mut found: Vec<String> = rows
        .as_array()
        .ok_or_else(|| format!("{query} returned {rows}"))?
        .iter()
        .filter_map(|r| r["name"].as_str().map(str::to_string))
        .collect();
    found.sort();
    Ok(found)
}

/// The `field` values of the `entity_type` items of a dead code result.
fn items(result: &Value, entity_type: &str, field: &str) -> Vec<String> {
    let mut found: Vec<String> = result["results"][entity_type]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .filter_map(|i| i[field].as_str().map(str::to_string))
        .collect();
    found.sort();
    found
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

/// The `edge_type` edges as (from name, to name).
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

#[path = "dart_dead_code_false_positive_test/exclude_extension_declarations_from.rs"]
mod exclude_extension_declarations_from;
#[path = "dart_dead_code_false_positive_test/exclude_flutter_platform_dirs.rs"]
mod exclude_flutter_platform_dirs;
#[path = "dart_dead_code_false_positive_test/exclude_test_file_functions.rs"]
mod exclude_test_file_functions;
#[path = "dart_dead_code_false_positive_test/recognize_stateful_widget_state.rs"]
mod recognize_stateful_widget_state;
