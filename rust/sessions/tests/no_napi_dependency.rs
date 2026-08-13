//! RPC-044 dependency-rule regression tests for `codelet-sessions`.
//!
//! Feature: spec/features/codelet-sessions-no-napi-regression.feature
//! Feature: spec/features/dependency-rule-regression-tests.feature (RPC-067 migration)
//!
//! These tests codify the architectural invariant that the
//! `codelet-sessions` crate — which the `fspec` binary wires into its
//! agent surface — must NOT transitively depend on `codelet-napi`. The
//! existing
//! `scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi`
//! in `tests/skeleton_invariants.rs` already enforces the cargo-metadata
//! half of this invariant (RPC-038); this file enforces the symmetric
//! source-import scan so the forbidden arrow stays absent in source as
//! well as in the resolved graph.
//!
//! RPC-067 migrated the cargo-metadata BFS walk and the source-tree
//! scan into the shared `codelet-test-helpers` crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-sessions --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the test parses `cargo metadata` output for codelet-sessions
    // @step And the resulting transitive package set does not contain the package name `codelet-napi`
    assert_no_transitive_dependency!("codelet-sessions", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-sessions --test no_napi_dependency`
    // @step And no `.rs` file under `rust/sessions/src/` contains the substring `codelet_napi`
    assert_no_import_in_sources!("sessions", "codelet_napi");
}
