//! Per-session token state — extracted from `store/agent_view.rs` in
//! RPC-099 so the parent module stays under the 300-LoC source-shape
//! ceiling enforced by `tests/source_shape_rpc024.rs`.
//!
//! Mirrors `ExtractedTokenState` from
//! `src/tui/utils/tokenStateUtils.ts`. The full TokenTracker shape
//! (codelet/rpc-types/src/lib.rs:766-788) is stored per-session so the
//! SessionHeader can display per-session reasoning_tokens,
//! tokens_per_second, and cache_read/cache_creation totals when the
//! user cycles sessions with Shift+Left/Right.

use codelet_rpc_types::{ContextFillInfo, StreamChunk, TokenTracker};

/// Per-session token state derived from `StreamChunk::TokenUpdate` +
/// `StreamChunk::ContextFillUpdate` events.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,
    /// RPC-099 — mirrors TokenTracker.cache_read_input_tokens.
    pub cache_read_input_tokens: u64,
    /// RPC-099 — mirrors TokenTracker.cache_creation_input_tokens.
    pub cache_creation_input_tokens: u64,
    /// RPC-099 — mirrors TokenTracker.reasoning_tokens. 0 means "no
    /// reasoning suffix" (matches the TS `reasoningTokens > 0` gate at
    /// src/tui/components/SessionHeader.tsx:196-197).
    pub reasoning_tokens: u64,
    /// RPC-099 — mirrors TokenTracker.tokens_per_second. Only rendered
    /// when the session is loading (build_right_line gate at
    /// header_build.rs:138-145).
    pub tokens_per_second: Option<f64>,
}

impl TokenState {
    /// Fold an arriving chunk into this state.
    pub fn apply_chunk(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::TokenUpdate { tokens } => self.apply_token_tracker(tokens),
            StreamChunk::ContextFillUpdate { context_fill } => {
                self.apply_context_fill(context_fill);
            }
            _ => {}
        }
    }

    fn apply_token_tracker(&mut self, t: &TokenTracker) {
        self.input_tokens = t.input_tokens as u64;
        self.output_tokens = t.output_tokens as u64;
        // RPC-099 — copy the four previously-dropped fields. Optionals
        // flatten to u64 (defaulting to 0) for counts so the render
        // site can compare `> 0` cheaply; `tokens_per_second` stays
        // Option because the loading-spinner segment uses None to
        // suppress the `N.N tok/s` prefix entirely.
        self.cache_read_input_tokens = t.cache_read_input_tokens.unwrap_or(0) as u64;
        self.cache_creation_input_tokens =
            t.cache_creation_input_tokens.unwrap_or(0) as u64;
        self.reasoning_tokens = t.reasoning_tokens.unwrap_or(0) as u64;
        self.tokens_per_second = t.tokens_per_second;
    }

    fn apply_context_fill(&mut self, info: &ContextFillInfo) {
        self.context_fill_pct = info.fill_percentage.min(100) as u8;
    }
}
