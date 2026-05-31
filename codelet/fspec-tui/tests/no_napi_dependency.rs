//! RPC-044 dependency-rule regression tests for `codelet-fspec-tui`.
//!
//! Feature: spec/features/codelet-fspec-tui-no-napi-regression.feature
//! Feature: spec/features/dependency-rule-regression-tests.feature (RPC-067 migration)
//!
//! These tests codify the architectural invariant that the `fspec-tui`
//! library — which the `fspec` binary embeds — must NOT transitively
//! depend on `codelet-napi`. The AgentView ultimately reaches
//! `codelet-sessions::SessionManager` through the `FspecBackend` trait,
//! never through the NAPI adapter.
//!
//! RPC-067 migrated the cargo-metadata BFS walk and the source-tree
//! scan into the shared `codelet-test-helpers` crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-fspec-tui --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the test parses `cargo metadata` output for codelet-fspec-tui
    // @step And the resulting transitive package set does not contain the package name `codelet-napi`
    assert_no_transitive_dependency!("codelet-fspec-tui", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step Given the RPC-044 changes are applied to the codelet workspace
    // @step When I run `cargo test -p codelet-fspec-tui --test no_napi_dependency`
    // @step And no `.rs` file under `codelet/fspec-tui/src/` contains the substring `codelet_napi`
    assert_no_import_in_sources!("fspec-tui", "codelet_napi");
}
