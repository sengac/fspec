//! Per-session token state — extracted from `store/agent_view.rs` in
//! RPC-099 so the parent module stays under the 300-LoC source-shape
//! ceiling enforced by `tests/source_shape_rpc024.rs`.
//!
//! Mirrors `ExtractedTokenState` from
//! `src/tui/utils/tokenStateUtils.ts`. The full TokenTracker shape
//! (rust/rpc-types/src/lib.rs:766-788) is stored per-session so the
//! SessionHeader can display per-session reasoning_tokens,
//! tokens_per_second, and cache_read/cache_creation totals when the
//! user cycles sessions with Shift+Left/Right.

use codelet_rpc_types::{ContextFillInfo, ContextProvenance, StreamChunk, TokenTracker};

/// Per-session token state derived from `StreamChunk::TokenUpdate` +
/// `StreamChunk::ContextFillUpdate` events.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TokenState {
    pub committed_generation: Option<i64>,
    pub compaction_generation: Option<i64>,
    pub context_request: Option<u64>,
    pub estimated: bool,
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
            StreamChunk::CommittedCompaction { snapshot } => {
                if self
                    .compaction_generation
                    .is_some_and(|current| current >= snapshot.generation)
                    || self
                        .committed_generation
                        .is_some_and(|current| current > snapshot.generation)
                {
                    return;
                }
                self.compaction_generation = Some(snapshot.generation);
                if self.committed_generation == Some(snapshot.generation)
                    && self.context_request.is_some()
                {
                    return;
                }
                self.apply_token_tracker(&snapshot.tokens);
                self.apply_context_fill(&snapshot.context_fill);
                self.committed_generation = Some(snapshot.generation);
                self.context_request = None;
                self.estimated = snapshot.estimated;
            }
            StreamChunk::ContextSnapshot { snapshot } => {
                if self.committed_generation.is_some_and(|generation| {
                    generation > snapshot.generation
                        || (generation == snapshot.generation
                            && self
                                .context_request
                                .is_some_and(|request| request >= snapshot.request))
                }) {
                    return;
                }
                self.committed_generation = Some(snapshot.generation);
                self.context_request = Some(snapshot.request);
                self.estimated = snapshot.provenance != ContextProvenance::Measured;
                if let Some(fill) = snapshot.context_fill() {
                    self.apply_context_fill(&fill);
                } else {
                    self.context_fill_pct = 0;
                    self.context_threshold_tokens = 0;
                }
            }
            StreamChunk::TokenUpdate { tokens } => self.apply_token_tracker(tokens),
            StreamChunk::ContextFillUpdate { context_fill }
                if self.committed_generation.is_none() =>
            {
                self.apply_context_fill(context_fill);
            }
            _ => {}
        }
    }

    fn apply_token_tracker(&mut self, t: &TokenTracker) {
        self.input_tokens = t.input_tokens as u64;
        self.output_tokens = t.output_tokens as u64;
        self.cache_read_input_tokens = t.cache_read_input_tokens.unwrap_or(0) as u64;
        self.cache_creation_input_tokens = t.cache_creation_input_tokens.unwrap_or(0) as u64;
        self.reasoning_tokens = t.reasoning_tokens.unwrap_or(0) as u64;
        self.tokens_per_second = t.tokens_per_second;
    }

    fn apply_context_fill(&mut self, info: &ContextFillInfo) {
        // RPC-100 — preserve the raw u32 fill_percentage (clamping only
        // to u16::MAX as a numeric range safeguard). The TS reference
        // explicitly notes the value can exceed 100% near the
        // compaction threshold; clamping to 100 would hide the
        // pre-compaction overshoot the user relies on.
        self.context_fill_pct = info.fill_percentage.min(u16::MAX as u32) as u16;
        self.context_threshold_tokens = if info.threshold.is_finite() && info.threshold > 0.0 {
            info.threshold as u64
        } else {
            self.context_threshold_tokens
        };
    }
}
