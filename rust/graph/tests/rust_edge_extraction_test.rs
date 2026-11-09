// Feature: spec/features/calls-imports-typeref-edges-rust.feature
//
// Integration tests for Rust Imports, Calls, and TypeRef edge extraction.
// Uses real Rust source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::{has_named_edge, index_fixture, named_edges};

/// A one-crate Cargo manifest, so the files reached from src/lib.rs are in
/// a module tree and resolve exactly.
const MANIFEST: (&str, &str) = ("Cargo.toml", "[package]\nname = \"fixture\"\n");

// ============================================================================
// Scenario: Extract Imports edges from Rust use statements
// ============================================================================
#[test]
fn test_rust_extract_imports_from_use_statements() {
    // @step Given a Rust file with `use crate::graph::helpers;`
    let rust_source = r#"
use crate::graph::helpers;
use serde_json::Value;

pub fn extract(source: &str) -> String {
    helpers::slugify_path(source)
}
"#;

    // @step And the target file `graph/helpers.rs` exists in the project
    let target_source = r#"
pub fn slugify_path(path: &str) -> String {
    path.replace('/', "-")
}
"#;

    // @step When the Rust extractor processes the source file
    // The crate's module tree (Cargo.toml, src/lib.rs, mod declarations)
    // states the exact project the `use` resolves in.
    let (_dir, indexed) = index_fixture(&[
        MANIFEST,
        ("src/lib.rs", "mod extractor;\nmod graph;\n"),
        ("src/graph.rs", "pub mod helpers;\n"),
        ("src/extractor.rs", rust_source),
        ("src/graph/helpers.rs", target_source),
    ]);
    let entities: Vec<GraphEntity> = indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to `graph-helpers-rs`
    assert!(
        has_named_edge(
            &entities,
            "Imports",
            "src/extractor.rs",
            "src/graph/helpers.rs"
        ),
        "Should have Imports edge to src/graph/helpers.rs. All Imports: {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And external `use serde_json::Value` imports should NOT produce edges
    let external_imports: Vec<_> = named_edges(&entities, "Imports")
        .into_iter()
        .filter(|(_, to)| to.contains("serde"))
        .collect();
    assert!(
        external_imports.is_empty(),
        "External serde_json imports should NOT produce edges, got: {:?}",
        external_imports
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Rust function calls
// ============================================================================
#[test]
fn test_rust_extract_calls_from_function_calls() {
    let rust_source = r#"
fn extract(source: &str) -> String {
    let slug = slugify_path(source);
    slug
}

fn slugify_path(path: &str) -> String {
    path.replace('/', "-")
}
"#;

    // @step Given a Rust file with function `extract()` that calls `slugify_path()`
    // @step And `slugify_path` is defined in the same file
    // @step When the Rust extractor processes the source file
    let (_dir, indexed) = index_fixture(&[MANIFEST, ("src/lib.rs", rust_source)]);
    let entities = indexed.entities;

    // @step Then a Calls edge should be emitted from `extract` to `slugify_path`
    assert!(
        has_named_edge(
            &entities,
            "Calls",
            "src/lib.rs::extract",
            "src/lib.rs::slugify_path"
        ),
        "Should have Calls edge from extract to slugify_path. All Calls: {:?}",
        named_edges(&entities, "Calls")
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from Rust type annotations
// ============================================================================
#[test]
fn test_rust_extract_typerefs_from_type_annotations() {
    let rust_source = r#"
pub struct GraphEntity {
    pub slug: String,
}

pub fn extract(source: &str) -> Vec<GraphEntity> {
    vec![]
}
"#;

    // @step Given a Rust file with `fn extract(source: &str) -> Vec<GraphEntity>`
    // @step And type `GraphEntity` is defined in the same file
    // @step When the Rust extractor processes the source file
    let (_dir, indexed) = index_fixture(&[MANIFEST, ("src/lib.rs", rust_source)]);
    let entities = indexed.entities;

    // @step Then a TypeRef edge should be emitted from `extract` to `GraphEntity`
    assert!(
        has_named_edge(
            &entities,
            "TypeRef",
            "src/lib.rs::extract",
            "src/lib.rs::GraphEntity"
        ),
        "Should have TypeRef edge from extract to GraphEntity. All TypeRef: {:?}",
        named_edges(&entities, "TypeRef")
    );
}
