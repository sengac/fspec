// Feature: spec/features/ast-extractor-edge-quality.feature
//
// Tests for edge quality fixes in Python, Java, and Go AST extractors.
// Every test indexes a fixture with the real pipeline; resolution is
// exact (an import resolves from the tree's import root, Go packages are
// directories with a package clause, Java by package declarations).
// Covers: suffix-matching for Imports resolution, Go import_map population,
// Go method-body Calls, Go/Python TypeRef extraction, Go same-package edges.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::{
    count_edges, has_named_edge, index_fixture, named_edges, write_test_file,
};

/// Index `root` with the real pipeline (one thread, no gitignore).
fn index(root: &std::path::Path) -> Indexed {
    index_tree(root, false, 1)
}

/// Index a project holding the one file `rel` with `source`.
fn index_one(rel: &str, source: &str) -> Vec<GraphEntity> {
    let (_dir, indexed) = index_fixture(&[(rel, source)]);
    indexed.entities
}

// ============================================================================
// Scenario: Python from-import statements produce Imports edges (suffix matching)
// ============================================================================

#[path = "ast_edge_quality_test/go_method_receiver_calls.rs"]
mod go_method_receiver_calls;
#[path = "ast_edge_quality_test/python_imports_with_prefix.rs"]
mod python_imports_with_prefix;

// ============================================================================
// Scenario: Python stdlib imports do not produce Imports edges
// ============================================================================

// ============================================================================
// Scenario: Java import declarations produce Imports edges (suffix matching)
// ============================================================================

// ============================================================================
// Scenario: Java imports with external-looking prefix resolve when file exists in project
// ============================================================================

// ============================================================================
// Scenario: Go same-package files have implicit Imports edges
// ============================================================================

// ============================================================================
// Scenario: Go method receiver bodies produce Calls edges
// ============================================================================

// ============================================================================
// Scenario: Go type references in function parameters produce TypeRef edges
// ============================================================================

// ============================================================================
// Scenario: Python type annotations produce TypeRef edges
// ============================================================================

// ============================================================================
// Scenario: Go cross-package import statements produce Imports edges
// ============================================================================
