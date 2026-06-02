//! RPC-101 — Context-fill percentage updates in real-time on every
//! TokenUpdate, not only on ContextFillUpdate.
//!
//! See `spec/work-units.json` work-unit RPC-101 for the full bug
//! description. In short: before this fix the SessionHeader `[X%]`
//! badge only repainted when a `StreamChunk::ContextFillUpdate`
//! arrived, which the backend may emit only at end-of-turn or skip
//! entirely on Esc interrupt. This left the percentage frozen during
//! the high-cadence `StreamChunk::TokenUpdate` stream that drives the
//! token counters and tok/s display.
//!
//! Fix: `TokenState::apply_context_fill` caches the threshold (in
//! tokens) on every ContextFillUpdate; `TokenState::apply_token_tracker`
//! re-derives `context_fill_pct` from the cached threshold and the
//! incoming TokenTracker's input_tokens + cache_read_input_tokens
//! using the same formula as the CLI emitter
//! (`emit_context_fill_from_usage` in
//! codelet/cli/src/interactive/stream_loop.rs:108-126).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::TokenState;
use codelet_rpc_types::{ContextFillInfo, StreamChunk, TokenTracker};

fn token_tracker(input: u32, output: u32, cache_read: u32) -> TokenTracker {
    TokenTracker {
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: Some(cache_read),
        cache_creation_input_tokens: Some(0),
        tokens_per_second: None,
        cumulative_billed_input: Some(0),
        cumulative_billed_output: Some(0),
        reasoning_tokens: None,
    }
}

fn context_fill(fill_pct: u32, threshold_tokens: f64) -> ContextFillInfo {
    ContextFillInfo {
        fill_percentage: fill_pct,
        effective_tokens: 0.0,
        threshold: threshold_tokens,
        context_window: 0.0,
    }
}

/// Without a prior ContextFillUpdate, a TokenUpdate alone must NOT
/// change `context_fill_pct` (we have no threshold to divide by).
#[test]
fn token_update_without_prior_context_fill_leaves_pct_unchanged() {
    // @step Given a fresh session with no ContextFillUpdate received yet (threshold cache is 0)
    let mut state = TokenState::default();
    // @step When a TokenUpdate with input_tokens=50000 arrives
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(50_000, 1_000, 0),
    });
    // @step Then the SessionHeader badge MUST remain at [0%] (no threshold means no recompute, never divide by zero)
    assert_eq!(state.context_fill_pct, 0);
    assert_eq!(state.context_threshold_tokens, 0);
    assert_eq!(state.input_tokens, 50_000);
}

/// After a ContextFillUpdate seeds the threshold, a subsequent
/// TokenUpdate MUST recompute `context_fill_pct` locally — this is
/// the core RPC-101 contract that makes the `[X%]` badge track
/// `tokens: X↓ Y↑` at the same cadence.
#[test]
fn token_update_after_context_fill_recomputes_percentage_locally() {
    let mut state = TokenState::default();

    // @step Given a session has received ContextFillUpdate with fill_percentage=10 and threshold=100000 tokens
    // Backend seeds threshold = 100_000 tokens (e.g. 200k context
    // window minus 32k output reservation, give or take cache fields).
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(10, 100_000.0),
    });
    assert_eq!(state.context_fill_pct, 10);
    assert_eq!(state.context_threshold_tokens, 100_000);

    // @step When a TokenUpdate with input_tokens=45000 arrives without an accompanying ContextFillUpdate
    // High-cadence TokenUpdate arrives mid-stream with no
    // accompanying ContextFillUpdate: 45_000 / 100_000 = 45%.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(45_000, 800, 0),
    });
    // @step Then the SessionHeader badge MUST display [45%] (recomputed locally from 45000/100000)
    assert_eq!(state.context_fill_pct, 45);

    // @step When a further TokenUpdate with input_tokens=90000 arrives later in the same turn
    // Another TokenUpdate further into the turn: 90_000 / 100_000 = 90%.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(90_000, 1_200, 0),
    });
    // @step Then the SessionHeader badge MUST display [90%] at TokenUpdate cadence
    assert_eq!(state.context_fill_pct, 90);
}

/// The local recompute MUST apply the 90% cache discount to
/// `cache_read_input_tokens` (matching `TokenTracker::effective_tokens`
/// in codelet/core/src/compaction/model.rs:90-96) so the badge
/// agrees with the backend's eventual ContextFillUpdate.
#[test]
fn recomputed_percentage_applies_90_percent_cache_discount() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens (from a prior ContextFillUpdate)
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(0, 100_000.0),
    });

    // @step When a TokenUpdate with input_tokens=50000 and cache_read_input_tokens=20000 arrives
    // input=50_000, cache_read=20_000.
    // effective = 50_000 - (20_000 * 0.9) = 50_000 - 18_000 = 32_000.
    // pct = round(32_000 / 100_000 * 100) = 32.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(50_000, 0, 20_000),
    });
    // @step Then the SessionHeader badge MUST display [32%] computed as effective=50000-(20000*0.9)=32000 and pct=round(32000/100000*100)=32 (matches TokenTracker.effective_tokens)
    assert_eq!(state.context_fill_pct, 32);
}

/// An authoritative ContextFillUpdate from the backend MUST override
/// any locally-recomputed value (backend is the source of truth when
/// it speaks).
#[test]
fn backend_context_fill_update_overrides_local_recompute() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens after a ContextFillUpdate{fill_percentage=5}
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(5, 100_000.0),
    });
    // @step Given a TokenUpdate with input_tokens=50000 has locally recomputed the badge to [50%]
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(50_000, 0, 0),
    });
    assert_eq!(state.context_fill_pct, 50);

    // @step When the backend emits an authoritative ContextFillUpdate{fill_percentage=62} (it knows about reasoning_tokens the local recompute does not model)
    // Backend disagrees (it knows about reasoning_tokens / other
    // adjustments the local recompute doesn't model) — its value wins.
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(62, 100_000.0),
    });
    // @step Then the SessionHeader badge MUST display [62%] (backend value wins)
    assert_eq!(state.context_fill_pct, 62);
    // @step Then the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates
    assert_eq!(state.context_threshold_tokens, 100_000);
}

/// The recompute MUST respect RPC-100's >100% overshoot rule — values
/// above 100 are valid and must NOT be clamped to 100.
#[test]
fn recomputed_percentage_preserves_overshoot_above_100() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(0, 100_000.0),
    });

    // @step When a TokenUpdate with input_tokens=110000 arrives
    // 110_000 / 100_000 = 110%.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(110_000, 0, 0),
    });
    // @step Then the SessionHeader badge MUST display [110%] (NOT clamped to 100 — the pre-compaction overshoot signal is preserved)
    assert_eq!(state.context_fill_pct, 110);
}

/// A ContextFillUpdate carrying a non-positive or non-finite threshold
/// (legacy fixture path — see `tests/agentview_session_header_..._rpc100.rs`
/// helper `context_fill(pct)` which sets `threshold: 0.0`) must NOT
/// wipe a previously-cached good threshold. Without this guard, every
/// existing RPC-100 test that passes `threshold: 0.0` would also
/// silently disable the RPC-101 recompute.
#[test]
fn zero_threshold_in_context_fill_does_not_wipe_cached_threshold() {
    let mut state = TokenState::default();
    // @step Given a session has received ContextFillUpdate{fill_percentage=50, threshold=100000}
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(50, 100_000.0),
    });
    assert_eq!(state.context_threshold_tokens, 100_000);

    // @step When a subsequent fixture-style ContextFillUpdate{fill_percentage=60, threshold=0.0} arrives (older fixtures only set fill_percentage)
    // Subsequent fixture-style ContextFillUpdate with threshold: 0.0
    // (common in unit tests that only care about fill_percentage).
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(60, 0.0),
    });
    // @step Then the badge MUST display [60%] from the new fill_percentage
    assert_eq!(state.context_fill_pct, 60);
    // @step Then the cached threshold MUST remain at 100000 tokens (non-positive threshold MUST NOT erase cached value)
    assert_eq!(
        state.context_threshold_tokens, 100_000,
        "non-positive threshold must not erase cached value"
    );

    // @step When a TokenUpdate with input_tokens=75000 arrives
    // And a TokenUpdate still recomputes against the cached threshold.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(75_000, 0, 0),
    });
    // @step Then the badge MUST recompute against the cached threshold and display [75%]
    assert_eq!(state.context_fill_pct, 75);
}
