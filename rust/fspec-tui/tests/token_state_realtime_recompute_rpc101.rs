//! Feature: spec/features/context-fill-percentage-realtime-recompute.feature
//!
//! RPC-101 — Context-fill percentage updates in real-time on every
//! TokenUpdate, not only on ContextFillUpdate.
//! RPC-419 — the local recompute formula is corrected from the
//! compaction cost proxy (input − 0.9·cache_read, rounded) to the
//! backend's physical-occupancy formula:
//!
//!   pct = trunc((input_tokens + output_tokens + reasoning_tokens)
//!               / threshold * 100)
//!
//! Wire `input_tokens` ALREADY includes cache_read + cache_creation
//! (PROV-001 total_input) so nothing cache-related is added or
//! subtracted; missing optional fields count as 0; truncation matches
//! the backend's `as u32` cast in `emit_context_fill_from_usage`
//! (rust/cli/src/interactive/stream_loop.rs:119-137) +
//! `ApiTokenUsage::total_context` (rust/core/src/token_usage.rs:64-72).
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.

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

/// Full-shape tracker for RPC-419 scenarios that exercise output,
/// reasoning, and optional cache fields explicitly.
fn token_tracker_full(
    input: u32,
    output: u32,
    cache_read: Option<u32>,
    cache_creation: Option<u32>,
    reasoning: Option<u32>,
) -> TokenTracker {
    TokenTracker {
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
        tokens_per_second: None,
        cumulative_billed_input: Some(0),
        cumulative_billed_output: Some(0),
        reasoning_tokens: reasoning,
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

/// Scenario: TokenUpdate without prior ContextFillUpdate leaves the
/// badge unchanged.
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

/// Scenario: TokenUpdate after cached threshold recomputes the badge
/// locally without a new ContextFillUpdate — the core RPC-101 contract
/// that makes the `[X%]` badge track `tokens: X↓ Y↑` at the same
/// cadence.
#[test]
fn token_update_after_context_fill_recomputes_percentage_locally() {
    let mut state = TokenState::default();

    // @step Given a session has received ContextFillUpdate with fill_percentage=10 and threshold=100000 tokens
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(10, 100_000.0),
    });
    assert_eq!(state.context_fill_pct, 10);
    assert_eq!(state.context_threshold_tokens, 100_000);

    // @step When a TokenUpdate with input_tokens=45000 arrives without an accompanying ContextFillUpdate
    // High-cadence TokenUpdate arrives mid-stream with no
    // accompanying ContextFillUpdate: 45_000 / 100_000 = 45%.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(45_000, 0, 0),
    });
    // @step Then the SessionHeader badge MUST display [45%] (recomputed locally from 45000/100000)
    assert_eq!(state.context_fill_pct, 45);

    // @step When a further TokenUpdate with input_tokens=90000 arrives later in the same turn
    // Another TokenUpdate further into the turn: 90_000 / 100_000 = 90%.
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(90_000, 0, 0),
    });
    // @step Then the SessionHeader badge MUST display [90%] at TokenUpdate cadence
    assert_eq!(state.context_fill_pct, 90);
}

/// Scenario: Local recompute uses the backend physical-occupancy
/// formula including output and reasoning tokens (RPC-419).
///
/// Old (wrong) formula ignored output + reasoning entirely and would
/// leave the badge at 50% here.
#[test]
fn local_recompute_uses_physical_occupancy_formula() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens (from a prior ContextFillUpdate)
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(0, 100_000.0),
    });

    // @step When a TokenUpdate with input_tokens=50000, output_tokens=3000 and reasoning_tokens=2000 arrives
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker_full(50_000, 3_000, Some(0), Some(0), Some(2_000)),
    });
    // @step Then the SessionHeader badge MUST display [55%] computed as trunc((50000+3000+2000)/100000*100) with no cache discount applied
    assert_eq!(
        state.context_fill_pct, 55,
        "pct must be trunc((50000+3000+2000)/100000*100) = 55 — output and reasoning tokens count, no cache discount"
    );
}

/// Scenario: Cache-heavy TokenUpdate no longer collapses the badge
/// (oscillation regression) — the RPC-419 bug itself.
///
/// Old (wrong) formula: 175000 − floor(150000·0.9) = 40000 →
/// round(40000/168000·100) = 24 — the badge sawtoothed 110% ↔ 24%.
#[test]
fn cache_heavy_token_update_does_not_collapse_badge() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=168000 tokens and an authoritative ContextFillUpdate showing 110%
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(110, 168_000.0),
    });
    assert_eq!(state.context_fill_pct, 110);

    // @step When a bare TokenUpdate arrives with input_tokens=175000 (including cache_read_input_tokens=150000 and cache_creation_input_tokens=5000), output_tokens=3000 and reasoning_tokens=8000
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker_full(175_000, 3_000, Some(150_000), Some(5_000), Some(8_000)),
    });
    // @step Then the SessionHeader badge MUST remain [110%] computed as trunc(186000/168000*100) instead of collapsing to [24%]
    assert_eq!(
        state.context_fill_pct, 110,
        "badge must remain at trunc(186000/168000*100) = 110, not collapse to 24 via the cache-discount formula"
    );
}

/// Scenario: Local recompute truncates like the backend instead of
/// rounding (RPC-419) — matches the backend's `as u32` cast.
#[test]
fn local_recompute_truncates_instead_of_rounding() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(0, 100_000.0),
    });

    // @step When a TokenUpdate with input_tokens=45900 and no output or reasoning tokens arrives
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker_full(45_900, 0, Some(0), Some(0), None),
    });
    // @step Then the SessionHeader badge MUST display [45%] (truncation matching the backend's `as u32` cast, not [46%] from rounding)
    assert_eq!(
        state.context_fill_pct, 45,
        "45900/100000*100 = 45.9 must truncate to 45 (backend `as u32` semantics), not round to 46"
    );
}

/// Scenario: Missing optional token fields are treated as zero
/// (RPC-419) — reasoning_tokens and cache counters absent on the wire.
#[test]
fn missing_optional_token_fields_treated_as_zero() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(0, 100_000.0),
    });

    // @step When a TokenUpdate with input_tokens=40000, output_tokens=1000 and absent reasoning and cache fields arrives
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker_full(40_000, 1_000, None, None, None),
    });
    // @step Then the SessionHeader badge MUST display [41%] without error
    assert_eq!(
        state.context_fill_pct, 41,
        "trunc((40000+1000+0)/100000*100) = 41 — absent optional fields count as zero"
    );
}

/// Scenario: Authoritative ContextFillUpdate overrides any
/// locally-recomputed value (backend is the source of truth when it
/// speaks).
#[test]
fn backend_context_fill_update_overrides_local_recompute() {
    let mut state = TokenState::default();
    // @step Given a session with cached threshold=100000 tokens after a ContextFillUpdate{fill_percentage=5}
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(5, 100_000.0),
    });
    // @step And a TokenUpdate with input_tokens=50000 has locally recomputed the badge to [50%]
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(50_000, 0, 0),
    });
    assert_eq!(state.context_fill_pct, 50);

    // @step When the backend emits an authoritative ContextFillUpdate{fill_percentage=62} (the backend remains authoritative whenever it speaks)
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(62, 100_000.0),
    });
    // @step Then the SessionHeader badge MUST display [62%] (backend value wins)
    assert_eq!(state.context_fill_pct, 62);
    // @step And the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates
    assert_eq!(state.context_threshold_tokens, 100_000);
}

/// Scenario: Local recompute preserves overshoot above 100%
/// (RPC-100 invariant) — values above 100 are valid and must NOT be
/// clamped to 100.
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

/// Scenario: ContextFillUpdate with non-positive threshold does not
/// wipe a previously-cached good threshold (legacy fixture path).
#[test]
fn zero_threshold_in_context_fill_does_not_wipe_cached_threshold() {
    let mut state = TokenState::default();
    // @step Given a session has received ContextFillUpdate{fill_percentage=50, threshold=100000}
    state.apply_chunk(&StreamChunk::ContextFillUpdate {
        context_fill: context_fill(50, 100_000.0),
    });
    assert_eq!(state.context_threshold_tokens, 100_000);

    // @step When a subsequent fixture-style ContextFillUpdate{fill_percentage=60, threshold=0.0} arrives (older fixtures only set fill_percentage)
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
    state.apply_chunk(&StreamChunk::TokenUpdate {
        tokens: token_tracker(75_000, 0, 0),
    });
    // @step Then the badge MUST recompute against the cached threshold and display [75%]
    assert_eq!(state.context_fill_pct, 75);
}
