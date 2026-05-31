//! RPC-092: codelet-agent-loop must not transitively depend on codelet-napi
//! after the codelet-graph lift. This guards the forbidden-arrow invariant
//! that was the WHOLE REASON for RPC-092 (so RPC-072 Phase B can host
//! deep_search_handler + graph_search_handler).
//!
//! Feature: spec/features/codelet-graph-crate-lift.feature
//!
//! Mirrors the RPC-067 pattern from
//! codelet/{core,sessions,fspec,fspec-tui,rpc-types}/tests/no_napi_dependency.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // RPC-092 lift contract:
    // @step Given the lift has been completed
    // @step When I add `codelet-graph = { workspace = true }` to codelet/agent-loop/Cargo.toml
    // @step Then `cargo test -p codelet-agent-loop --test no_napi_dependency` still passes
    //
    // RPC-072 Phase A foundation refit contract:
    // @step Given the codelet-agent-loop crate exists under codelet/agent-loop/
    // @step When cargo metadata is invoked for the codelet-agent-loop package
    // @step Then the transitive package set does not contain "codelet-napi"
    assert_no_transitive_dependency!("codelet-agent-loop", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // RPC-092 lift contract:
    // @step Given the codelet-graph lift introduces a new dep edge codelet-agent-loop → codelet-graph
    // @step When I scan codelet/agent-loop/src for codelet_napi imports
    // @step Then no .rs file under codelet/agent-loop/src/ contains the substring `codelet_napi`
    //
    // RPC-072 Phase A foundation refit contract:
    // @step And no .rs file under codelet/agent-loop/src/ contains the substring "codelet_napi"
    assert_no_import_in_sources!("agent-loop", "codelet_napi");
}
