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

use codelet_fspec_tui::store::TokenState;
use codelet_rpc_types::{ContextFillInfo, StreamChunk, TokenTracker};
use codelet_test_helpers::TestResult;

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
fn token_update_without_prior_context_fill_leaves_pct_unchanged() -> TestResult {
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
    Ok(())
}

/// Scenario: Cache-heavy TokenUpdate no longer collapses the badge
/// (oscillation regression) — the RPC-419 bug itself.
///
/// Old (wrong) formula: 175000 − floor(150000·0.9) = 40000 →
/// round(40000/168000·100) = 24 — the badge sawtoothed 110% ↔ 24%.
#[test]
fn cache_heavy_token_update_does_not_collapse_badge() -> TestResult {
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
    Ok(())
}
