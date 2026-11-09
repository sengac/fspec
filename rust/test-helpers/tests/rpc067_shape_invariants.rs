//! RPC-067 shape-invariant regression tests.
//!
//! Feature: spec/features/dependency-rule-regression-tests.feature
//!
//! These tests codify the structural shape of the RPC-067 migration:
//! every scenario in the feature file that describes a "this file
//! contains exactly two #[test] fns delegating to the helper" or
//! "this Cargo.toml declares codelet-test-helpers under
//! [dev-dependencies]" or "this Cargo.toml lists test-helpers as a
//! workspace member" is enforced here, so a future refactor that
//! widens / collapses one of these files trips the test.
//!
//! The cargo-metadata + source-walk runtime invariants are covered by
//! the five per-crate `no_napi_dependency.rs` files. This file is
//! ONLY about the migration shape.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// Workspace root: `rust/` (one level above this test crate).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test-helpers manifest dir must have a parent")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = workspace_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[path = "rpc067_shape_invariants/codelet_test_helpers_crate.rs"]
mod codelet_test_helpers_crate;
#[path = "rpc067_shape_invariants/source_scan_helper_ignores.rs"]
mod source_scan_helper_ignores;
