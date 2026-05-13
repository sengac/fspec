//! RPC-007: Deterministic test-only LLM stub provider.
//!
//! Behind the `test-support` Cargo feature so production builds never
//! compile a stub provider into release artifacts. Emits a deterministic
//! `[StreamChunk::Text("hi back"), StreamChunk::Done]` sequence on any
//! send_input regardless of input. Both transports' integration tests
//! enable this feature and assert byte-equal chunks across embedded and
//! WebSocket paths.
//!
//! Any pre-existing mock in codelet/providers stays untouched — this is a
//! fresh, minimal stub focused exclusively on the cross-transport parity
//! tests.

use codelet_rpc_types::StreamChunk;

/// Minimal deterministic provider used by RPC-007 cross-transport tests.
///
/// The provider is intentionally trivial: it does not implement
/// [`crate::LlmProvider`] (which has a much larger surface). It exposes
/// a single [`StubProvider::canned_chunks`] method that returns the
/// deterministic sequence the cross-transport parity tests assert on.
#[derive(Debug, Default, Clone)]
pub struct StubProvider;

impl StubProvider {
    pub fn new() -> Self {
        Self
    }

    /// Return the deterministic chunk sequence emitted on any input.
    pub fn canned_chunks() -> Vec<StreamChunk> {
        vec![StreamChunk::text("hi back".to_string()), StreamChunk::done()]
    }
}
