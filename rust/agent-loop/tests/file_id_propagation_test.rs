#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/file-id-propagation-through-dag.feature
//!
//! This test file validates the acceptance criteria for CMPCT-021:
//! File ID Propagation Through DAG.
//!
//! Tests verify:
//! - build_dag_files_block merges annotations and existing dag-files
//! - parse_dag_files_block extracts (path, operation) tuples
//! - apply_pending_dag auto-appends dag-files block when agent omits it
//! - Agent-provided dag-files blocks are preserved without duplication
//! - Incremental compaction instruction mentions dag-files preservation
//! - Fresh compaction instruction mentions Active Files section

use codelet_agent_loop::inject_summary_handler;
use codelet_core::compaction::{FileOp, StructuralAnnotation};

#[path = "file_id_propagation_test/dag_files_block.rs"]
mod dag_files_block;
#[path = "file_id_propagation_test/parsing_and_instructions.rs"]
mod parsing_and_instructions;
