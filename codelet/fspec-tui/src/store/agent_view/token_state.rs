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
    /// RPC-100 — widened to u16 so values >100% (the pre-compaction
    /// overshoot signal documented in TS `SessionHeader.tsx:101`:
    /// "Fill percentage (0-100+, can exceed 100 near compaction
    /// threshold)") survive the wire→store transition. Previously a
    /// `u8` with a `.min(100)` clamp that collapsed 105% to 100%.
    pub context_fill_pct: u16,
    /// RPC-101 — cached context-fill threshold (in tokens) captured from
    /// the last `StreamChunk::ContextFillUpdate`. Used by
    /// `apply_token_tracker` to recompute `context_fill_pct` in
    /// real-time on every `TokenUpdate` instead of waiting for the
    /// backend's next `ContextFillUpdate`. `0` means "no threshold
    /// known yet" — the recompute is skipped until at least one
    /// `ContextFillUpdate` has arrived for this session.
    pub context_threshold_tokens: u64,
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
        self.cache_creation_input_tokens = t.cache_creation_input_tokens.unwrap_or(0) as u64;
        self.reasoning_tokens = t.reasoning_tokens.unwrap_or(0) as u64;
        self.tokens_per_second = t.tokens_per_second;

        // RPC-101 — recompute context_fill_pct in real-time off the
        // incoming TokenTracker so the SessionHeader `[X%]` badge
        // updates at the same cadence as the `tokens: X↓ Y↑` counters.
        // Without this, the badge only refreshes on ContextFillUpdate
        // chunks (which the backend may emit only at end-of-turn or
        // skip entirely when the user presses Esc mid-stream),
        // leaving a stale percentage visible for the rest of the turn.
        //
        // Formula mirrors `emit_context_fill_from_usage` in
        // codelet/cli/src/interactive/stream_loop.rs:108-126 and the
        // TS reference `calculateContextFillPercentage` in
        // src/tui/utils/tokenStateUtils.ts:103-117. Threshold is
        // captured from the last ContextFillUpdate (which IS the
        // authoritative source for context_window - max_output_reservation);
        // a real ContextFillUpdate will overwrite this value next.
        if self.context_threshold_tokens > 0 {
            // Cache discount: cache_read tokens cost 10% of normal
            // (matches `TokenTracker::effective_tokens` in
            // codelet/core/src/compaction/model.rs:90-96).
            let cache_discount = ((self.cache_read_input_tokens as f64) * 0.9) as u64;
            let effective = self.input_tokens.saturating_sub(cache_discount);
            let pct = ((effective as f64 / self.context_threshold_tokens as f64) * 100.0).round();
            self.context_fill_pct = if pct.is_finite() && pct >= 0.0 {
                pct.min(u16::MAX as f64) as u16
            } else {
                0
            };
        }
    }

    fn apply_context_fill(&mut self, info: &ContextFillInfo) {
        // RPC-100 — preserve the raw u32 fill_percentage (clamping only
        // to u16::MAX as a numeric range safeguard). The TS reference
        // explicitly notes the value can exceed 100% near the
        // compaction threshold; clamping to 100 would hide the
        // pre-compaction overshoot the user relies on.
        self.context_fill_pct = info.fill_percentage.min(u16::MAX as u32) as u16;
        // RPC-101 — cache the threshold (in tokens) so subsequent
        // TokenUpdate chunks can recompute the percentage locally
        // without waiting for the next ContextFillUpdate. Negative /
        // non-finite values are treated as "unknown" (threshold = 0).
        self.context_threshold_tokens = if info.threshold.is_finite() && info.threshold > 0.0 {
            info.threshold as u64
        } else {
            self.context_threshold_tokens
        };
    }
}
