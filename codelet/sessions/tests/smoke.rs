//! Smoke test for the `codelet-sessions` crate skeleton (RPC-038).
//!
//! Feature: spec/features/codelet-sessions-crate-skeleton.feature
//!
//! Locks in the compile-only contract for the empty crate. RPC-039 and
//! RPC-040 will populate the `background_session` and `session_manager`
//! modules; until then, the crate's only job is to compile and to be
//! reachable as a workspace member.

/// Smoke test demanded by acceptance criterion #6 in the RPC-038
/// attachment (`codelet-sessions-skeleton.md`). The body is empty by
/// design — the test passes iff the crate's test binary links.
#[test]
fn crate_compiles() {}
