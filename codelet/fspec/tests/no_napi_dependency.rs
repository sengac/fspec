//! RPC-044 dependency-rule regression tests for `codelet-fspec`.
//!
//! Feature: spec/features/codelet-fspec-no-napi-regression.feature
//! Feature: spec/features/dependency-rule-regression-tests.feature (RPC-067 migration)
//!
//! These tests codify the architectural invariant that the `fspec` binary
//! must NOT transitively depend on `codelet-napi`. The agent surface
//! reaches the binary through the NAPI-free `codelet-sessions` crate;
//! `codelet-napi` is reserved purely for the JS bridge.
//!
//! RPC-067 migrated the cargo-metadata BFS walk and the source-tree
//! scan into the shared `codelet-test-helpers` crate. This file is now a
//! thin wrapper that names the crate + forbidden module / package.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // RPC-044 contract:
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-fspec --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the test parses `cargo metadata` output for codelet-fspec
    // @step And the resulting transitive package set does not contain the package name `codelet-napi`
    //
    // RPC-072 Phase A boundary refit contract:
    // @step Given the codelet-fspec build_service still installs FspecAgentHooks from codelet-agent-loop
    // @step When cargo metadata is invoked for the codelet-fspec package
    // @step Then the transitive package set does not contain "codelet-napi"
    assert_no_transitive_dependency!("codelet-fspec", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-fspec --test no_napi_dependency`
    // @step And no `.rs` file under `codelet/fspec/src/` contains the substring `codelet_napi`
    assert_no_import_in_sources!("fspec", "codelet_napi");
}
