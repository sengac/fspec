//! Shared test-only helpers for the codelet workspace.
//!
//! This crate is the home for utilities that multiple test binaries need
//! to share — most importantly the architectural-invariant assertions
//! that pin the workspace's forbidden-arrow rules (RPC-002, RPC-006,
//! RPC-044, RPC-067).
//!
//! The crate has zero workspace-internal dependencies on purpose: every
//! test binary in the workspace must be able to consume it without
//! transitively pulling forbidden crates like `codelet-napi`.
//!
//! # Module map
//!
//! - [`dependency_rules`] — generic dependency-rule helpers that walk
//!   `cargo metadata` and source trees to enforce arrows like
//!   `codelet-fspec → codelet-napi` are absent. Used by every
//!   `tests/no_napi_dependency.rs` regression test in the workspace.

pub mod dependency_rules;
