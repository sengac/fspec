// Feature: spec/features/calls-imports-typeref-edges-rust.feature
//
// Integration tests for Rust Imports, Calls, and TypeRef edge extraction.
// Uses real Rust source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_rust_extractor::extract_rust;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Rust use statements
// ============================================================================
#[test]
fn test_rust_extract_imports_from_use_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Rust file with `use crate::graph::helpers;`
    let rust_source = r#"
use crate::graph::helpers;
use serde_json::Value;

pub fn extract(source: &str) -> String {
    helpers::slugify_path(source)
}
"#;
    write_test_file(project_dir, "src/extractor.rs", rust_source);

    // @step And the target file `graph/helpers.rs` exists in the project
    let target_source = r#"
pub fn slugify_path(path: &str) -> String {
    path.replace('/', "-")
}
"#;
    write_test_file(project_dir, "src/graph/helpers.rs", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Rust extractor processes the source file
    let entities = extract_rust(rust_source, "src/extractor.rs", &known_files)
        .expect("Rust extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to `graph-helpers-rs`
    let local_imports = find_edges(
        &entities,
        "Imports",
        Some("src-extractor-rs"),
        Some("graph"),
    );
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge to graph/helpers.rs. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `use serde_json::Value` imports should NOT produce edges
    let external_imports = find_edges(&entities, "Imports", None, Some("serde"));
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
    let known_files = HashSet::new();

    // @step Given a Rust file with function `extract()` that calls `slugify_path()`
    // @step And `slugify_path` is defined in the same file
    // @step When the Rust extractor processes the source file
    let entities = extract_rust(rust_source, "src/lib.rs", &known_files)
        .expect("Rust extraction should succeed");

    // @step Then a Calls edge should be emitted from `extract` to `slugify_path`
    let calls = find_edges(&entities, "Calls", Some("extract"), Some("slugify_path"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from extract to slugify_path. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
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
    let known_files = HashSet::new();

    // @step Given a Rust file with `fn extract(source: &str) -> Vec<GraphEntity>`
    // @step And type `GraphEntity` is defined in the same file
    // @step When the Rust extractor processes the source file
    let entities = extract_rust(rust_source, "src/lib.rs", &known_files)
        .expect("Rust extraction should succeed");

    // @step Then a TypeRef edge should be emitted from `extract` to `GraphEntity`
    let typerefs = find_edges(&entities, "TypeRef", Some("extract"), Some("GraphEntity"));
    assert!(
        !typerefs.is_empty(),
        "Should have TypeRef edge from extract to GraphEntity. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );
}
