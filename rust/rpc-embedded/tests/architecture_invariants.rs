//! Source-shape integration tests for RPC-005 architecture invariants.
//!
//! Feature: spec/features/rpc-architecture-invariants.feature
//!
//! These tests codify architectural invariants statically by inspecting
//! source files in the workspace. They do not exercise any runtime code
//! path.
//!
//! - Scenario 7: EmbeddedTransport requires a tokio runtime Handle at construction
//! - Scenario 8: WorkUnitInfo is defined once in rpc-types and re-exported by codelet-napi
//! - Scenario 9: Embedded transport uses only tarpc::transport::channel for in-process traffic
//! - Scenario 10: Shared service crate codelet-rpc has no dependency on codelet-core or the NAPI work_units_watcher
//! - Scenario 11: rpc-server binary binds to 127.0.0.1 loopback only

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod source_helpers;

use source_helpers::{
    collect_rs_files, read_to_string_or_panic, strip_rust_comments, workspace_root,
};

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> String {
    codelet_test_helpers::source_composition::read_module_tree(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[path = "architecture_invariants/scenario_10_codelet_rpc_has.rs"]
mod scenario_10_codelet_rpc_has;
#[path = "architecture_invariants/scenario_7_embedded_transport_requires.rs"]
mod scenario_7_embedded_transport_requires;
