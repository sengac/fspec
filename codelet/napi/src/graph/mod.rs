//! Graph Database Module — Dual-Graph Architecture
//!
//! Provides embedded nanograph property graph databases for the dual-graph
//! architecture:
//!
//! 1. **AST Code Graph** (`"ast-code"`) — Code structure, dependencies, and relationships
//!    stored at `<project>/.fspec/graph/ast-code.nano/`
//! 2. **Learnings Graph** (`"learnings"`) — Accumulated knowledge, decisions, and conventions
//!    stored at `<project>/.fspec/graph/learnings.nano/`
//!
//! Uses a registry of named graph instances (see `registry.rs`).

/// Close all graph databases cleanly.
///
/// Should be called on process exit to avoid Lance corruption.
pub fn close_graph_db() {
    registry::close_all_graphs();
}

/// Reset all graph databases.
///
/// Called when the data directory changes (via `set_data_directory()`).
pub fn reset_graph_db() {
    registry::reset_all_graphs();
}

pub mod ast_dispatch;
pub mod ast_pipeline;
pub mod database;
pub mod dispatch_helpers;
pub mod graph_entities;
pub mod learnings_context;
pub mod learnings_dispatch;
pub mod learnings_extraction;
pub mod llm_response_parser;
pub mod registry;
