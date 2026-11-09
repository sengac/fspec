// Feature: spec/features/ast-graph-data-model.feature
//
// AST Graph Data Model & Nanograph Schema
// Tests for the reusable GraphDatabase abstraction, the AST-code schema,
// and the registry's multi-instance support.
//
// Each test uses an isolated temp directory to avoid polluting real data.
// The code entities loaded come from indexing a real fixture with the fact
// pipeline (slugs are paths and `<qualifiedName>@<start byte>`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::{entities_to_jsonl, GraphEntity};
use serde_json::Value;

pub mod graph_test_helpers;
use graph_test_helpers::{index_fixture, node_names};

/// The node types of the AST code schema.
const AST_NODE_TYPES: &[&str] = &["File", "Function", "Type", "Variable", "Dependency"];

/// The entities of `entities` that are nodes of one of `node_types`.
fn nodes_of(entities: &[GraphEntity], node_types: &[&str]) -> Vec<GraphEntity> {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_types.contains(&node_type.as_str())))
        .cloned()
        .collect()
}

/// The entities of `entities` that are edges of `edge_type`.
fn edges_of(entities: &[GraphEntity], edge_type: &str) -> Vec<GraphEntity> {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type: t, .. } if t == edge_type))
        .cloned()
        .collect()
}

/// The slug of the node named `name` (a File's path or a qualifiedName).
fn slug_named(entities: &[GraphEntity], name: &str) -> String {
    let found: Vec<String> = node_names(entities)
        .into_iter()
        .filter(|(_, n)| n == name)
        .map(|(slug, _)| slug)
        .collect();
    assert_eq!(found.len(), 1, "one node named {name}: {found:?}");
    found[0].clone()
}

/// The AST code schema, same as what's bundled in the binary.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// A minimal alternative schema for testing multi-instance separation.
const ALT_SCHEMA: &str = r#"
node Widget @description("A test widget node.") {
    slug: String @key
    label: String
}

edge LinksTo: Widget -> Widget {
}
"#;

/// Inline query source for traversing AST graph neighbors.
const AST_QUERIES: &str = r#"
query all_files() {
    match { $f: File }
    return { $f.slug, $f.path, $f.language, $f.lineCount, $f.isTest }
}

query all_functions() {
    match { $fn: Function }
    return { $fn.slug, $fn.name, $fn.qualifiedName, $fn.isAsync, $fn.isPublic, $fn.paramCount }
}

query file_functions($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f contains $fn
    }
    return { $fn.slug, $fn.name, $fn.qualifiedName }
}

query function_callees($fn_slug: String) {
    match {
        $caller: Function { slug: $fn_slug }
        $caller calls $callee
    }
    return { $callee.slug, $callee.name }
}

query function_callers($fn_slug: String) {
    match {
        $caller calls $target
        $target: Function { slug: $fn_slug }
    }
    return { $caller.slug, $caller.name }
}
"#;

#[path = "ast_graph_data_model_test/initialize_ast_graph_database.rs"]
mod initialize_ast_graph_database;
#[path = "ast_graph_data_model_test/reusable_graph_database_supports.rs"]
mod reusable_graph_database_supports;
