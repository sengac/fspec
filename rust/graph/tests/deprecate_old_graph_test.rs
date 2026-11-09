// Feature: spec/features/deprecate-old-graph-migrate-useful-data.feature
//
// Deprecate Old Graph & Migrate Useful Data
// Tests that the old monolithic agent-memory graph infrastructure has been
// completely removed, leaving only the lean dual-graph architecture
// (AST + Learnings).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::database::GraphDatabase;
use codelet_graph::registry;
use std::path::Path;

#[path = "deprecate_old_graph_test/graph_module_only_exports.rs"]
mod graph_module_only_exports;
#[path = "deprecate_old_graph_test/graphsearch_action_enum_only.rs"]
mod graphsearch_action_enum_only;
