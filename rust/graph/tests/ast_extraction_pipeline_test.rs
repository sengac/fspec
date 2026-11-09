// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
//
// AST Extraction Pipeline — Tree-Sitter/AST-Grep Parser
// Tests for extracting AST entities (Functions, Types, imports)
// from TypeScript and Rust source files using ast-grep patterns.
//
// Includes deduplication tests for KGRAPH-026: duplicate File entities
// from import resolution vs. direct file walk.
//
// Each test uses an isolated temp directory with synthetic source files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::facts::extract::extract_path;
use codelet_graph::ast_pipeline::{index_tree, walk_and_extract};
use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::{
    count_edges, count_nodes, has_named_edge, named_edges, node_named, write_test_file,
};

/// The AST code schema for loading extracted entities.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Inline queries for verifying loaded data.
const AST_QUERIES: &str = r#"
query all_files() {
    match { $f: File }
    return { $f.slug, $f.path, $f.language, $f.lineCount, $f.isTest }
}

query all_functions() {
    match { $fn: Function }
    return { $fn.slug, $fn.name, $fn.qualifiedName, $fn.isAsync, $fn.isPublic, $fn.paramCount, $fn.lineStart, $fn.lineEnd }
}

query all_types() {
    match { $t: Type }
    return { $t.slug, $t.name, $t.typeKind, $t.isPublic }
}

query file_functions($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f contains $fn
    }
    return { $fn.slug, $fn.name }
}

query file_imports($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f imports $target
    }
    return { $target.slug, $target.path }
}
"#;

#[path = "ast_extraction_pipeline_test/barrel_imports_resolve_to.rs"]
mod barrel_imports_resolve_to;
#[path = "ast_extraction_pipeline_test/entity_dedup_file_nodes.rs"]
mod entity_dedup_file_nodes;
#[path = "ast_extraction_pipeline_test/extract_function_nodes_from.rs"]
mod extract_function_nodes_from;
#[path = "ast_extraction_pipeline_test/multiple_importers_produce_single.rs"]
mod multiple_importers_produce_single;
#[path = "ast_extraction_pipeline_test/walk_project_directory_with.rs"]
mod walk_project_directory_with;

// ============================================================================
// KGRAPH-026: AST Entity Deduplication Tests
// Feature: spec/features/ast-entity-deduplication.feature
// ============================================================================
