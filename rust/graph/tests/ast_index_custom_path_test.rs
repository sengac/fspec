// Feature: spec/features/ast-extraction-pipeline.feature
//
// AST Index Custom Path — Tests for indexing external directories
// via the `path` parameter on `ast_index`. Verifies that:
// - `walk_and_extract` with `respect_gitignore: false` finds files
//   inside gitignored directories
// - `walk_and_extract` with `respect_gitignore: true` still skips them
// - `AstIndex` deserializes with and without the optional `path` field

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::walk_and_extract;
use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::{count_nodes, write_test_file};

#[path = "ast_index_custom_path_test/ast_index_deserializes_without.rs"]
mod ast_index_deserializes_without;
#[path = "ast_index_custom_path_test/walk_and_extract_extracts.rs"]
mod walk_and_extract_extracts;
