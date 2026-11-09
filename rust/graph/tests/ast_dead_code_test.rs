// Feature: spec/features/ast-dead-code-detection.feature
//
// Dead Code Detection via AST Graph — Calls/TypeRef Edge Population + Orphan Query
//
// Tests for:
// 1. Calls edge extraction (same-file and cross-file function calls)
// 2. TypeRef edge extraction (parameter and return type references)
// 3. Filtering out method calls and builtins from Calls edges
// 4. Dead code queries via nanograph anti-join (orphan files, uncalled functions, unreferenced types)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use codelet_graph::ast_dead_code::dispatch_ast_dead_code;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{edges, entities, index, project, slug_of};

/// A temp project holding `files`, indexed with `ast_index`; returns the
/// temp directory (keep it alive), the root and the graph's entities.
async fn indexed(
    files: &[(&str, &str)],
) -> Result<(tempfile::TempDir, PathBuf, Vec<GraphEntity>), TestError> {
    let (directory, root) = project(files)?;
    index(&root).await?;
    let graph = entities(&root).await?;
    Ok((directory, root, graph))
}

/// Whether an `edge_type` edge leads from the node of `from_type` named
/// `from` to the node of `to_type` named `to` (qualifiedNames).
fn has_edge(
    graph: &[GraphEntity],
    edge_type: &str,
    (from_type, from): (&str, &str),
    (to_type, to): (&str, &str),
) -> Result<bool, TestError> {
    let from = slug_of(graph, from_type, from)?;
    let to = slug_of(graph, to_type, to)?;
    Ok(edges(graph, edge_type)
        .iter()
        .any(|(f, t, _)| *f == from && *t == to))
}

/// The `field` values of the items `ast_dead_code` reports for
/// `entity_type` in `root`'s graph.
async fn dead_code(root: &Path, entity_type: &str, field: &str) -> Result<Vec<String>, TestError> {
    let graph = registry::read_code_graph(root).await?;
    let output = dispatch_ast_dead_code(&graph, Some(entity_type), None, None).await;
    let value: Value = serde_json::from_str(&output)?;
    let items = value["results"][entity_type]["items"]
        .as_array()
        .ok_or_else(|| format!("no {entity_type} items in {output}"))?;
    let mut found: Vec<String> = items
        .iter()
        .filter_map(|item| item[field].as_str().map(str::to_string))
        .collect();
    found.sort();
    Ok(found)
}

#[path = "ast_dead_code_test/detect_uncalled_functions_no.rs"]
mod detect_uncalled_functions_no;
#[path = "ast_dead_code_test/extract_calls_edge_cross.rs"]
mod extract_calls_edge_cross;
