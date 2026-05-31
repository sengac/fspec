//! RPC-067 dependency-rule regression tests for `codelet-core`.
//!
//! Feature: spec/features/dependency-rule-regression-tests.feature
//!
//! These tests codify the architectural invariant that the
//! `codelet-core` crate — pure-Rust agent execution — must NOT
//! transitively depend on `codelet-napi`. `codelet-core` sits below the
//! RPC service layer in the dependency hierarchy and must be reachable
//! from both the JS bridge (`codelet-napi`) and the pure-Rust bridge
//! (`codelet-sessions`) without taking the JS bridge as a dependency.
//!
//! The work is delegated to the shared `codelet-test-helpers` crate
//! (RPC-067) which centralises the cargo-metadata BFS walk and the
//! source-tree scan. Each #[test] is a one-liner that names the crate
//! and the forbidden module / package.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // @step Given the codelet workspace is in its current RPC-067 state
    // @step When I run `cargo test -p codelet-core --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the transitive dependency walk for codelet-core does NOT contain codelet-napi
    assert_no_transitive_dependency!("codelet-core", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step Given the codelet workspace is in its current RPC-067 state
    // @step And no `.rs` file under codelet/core/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
    assert_no_import_in_sources!("core", "codelet_napi");
}
