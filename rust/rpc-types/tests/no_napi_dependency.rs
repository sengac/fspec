//! RPC-067 dependency-rule regression tests for `codelet-rpc-types`.
//!
//! Feature: spec/features/dependency-rule-regression-tests.feature
//!
//! These tests codify the architectural invariant that the
//! `codelet-rpc-types` crate — the shared wire-portable type surface
//! used by both the JS bridge and the pure-Rust bridge — must NOT
//! transitively depend on `codelet-napi`.
//!
//! Note: `codelet-rpc-types` exposes an optional `napi` cargo feature
//! that pulls in the third-party `napi` and `napi-derive` crates so the
//! same type definitions can be re-decorated with `#[napi(object)]` for
//! the JS bridge. That third-party `napi` crate is DIFFERENT from
//! `codelet-napi` (the workspace's JS bridge crate). The transitive
//! walk below targets `codelet-napi` only — the existence of the
//! `napi` feature does not violate the forbidden-arrow rule.
//!
//! This test is run with the default feature set (no `napi` feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // @step Given the codelet workspace is in its current RPC-067 state
    // @step And the codelet-rpc-types crate is built with the default feature set (no `napi` feature)
    // @step When I run `cargo test -p codelet-rpc-types --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the transitive dependency walk for codelet-rpc-types does NOT contain codelet-napi
    assert_no_transitive_dependency!("codelet-rpc-types", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step Given the codelet workspace is in its current RPC-067 state
    // @step And no `.rs` file under rust/rpc-types/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
    assert_no_import_in_sources!("rpc-types", "codelet_napi");
}
