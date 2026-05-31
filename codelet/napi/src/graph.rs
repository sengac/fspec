//! RPC-092: Thin re-export of codelet-graph for backward compatibility with
//! existing NAPI bindings and the codelet/napi/tests/ast_*_test.rs
//! integration test suite.
//!
//! New code in the workspace should depend on `codelet-graph` directly.
//! This shim exists solely so the codelet-napi crate keeps compiling after
//! the RPC-092 lift, without forcing 24 integration tests to be rewritten.

pub use codelet_graph::*;
