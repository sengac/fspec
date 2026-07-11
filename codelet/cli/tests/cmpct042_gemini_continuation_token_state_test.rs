#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/gemini-continuation-cache-double-count.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! CMPCT-042 root fix: the Gemini continuation TokenState at
//! gemini_continuation.rs:126-132 was seeded with the tracker's
//! cache-INCLUSIVE total (`session.token_tracker.input_tokens` after
//! `update_display_only`, which stores `usage.total_input()`) ALONGSIDE the
//! non-zero cache fields from `current_display` — so `TokenState::total()`
//! counted cache twice and the CompactionHook could trigger compaction early.
//! The fix routes the seed through the audited
//! `TokenState::from_cache_inclusive_total` constructor which zeroes the
//! cache fields (the 'Don't double count' invariant from
//! stream_loop.rs:398-401).

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use codelet_core::compaction::TokenTracker;
use codelet_core::{ApiTokenUsage, CompactionHook, TokenState};
use rig::agent::CancelSignal;

// ===========================================================================
// Helpers
// ===========================================================================

/// Drive a tracker through `update_display_only` with the CMPCT-042 worked
/// example: 30k fresh input + 150k cache_read = 180k cache-inclusive total,
/// 2k cumulative output. Mirrors gemini_continuation.rs:116-124.
fn tracker_after_display_update() -> TokenTracker {
    let mut tracker = TokenTracker::new();
    let usage = ApiTokenUsage::new(30_000, 150_000, 0, 0);
    tracker.update_display_only(&usage, 2_000);
    assert_eq!(
        tracker.input_tokens, 180_000,
        "sanity: update_display_only stores the cache-INCLUSIVE total"
    );
    assert_eq!(
        tracker.cache_read_input_tokens,
        Some(150_000),
        "sanity: tracker carries cache_read alongside the total"
    );
    tracker
}

/// Mirror the production `CompactionHook::on_completion_call` trigger
/// condition for synchronous tests (same mirror as
/// compaction_trigger_reliability_test.rs): when `state.total() > threshold`,
/// set `compaction_needed=true` and cancel the signal.
fn simulate_hook_check(
    hook: &CompactionHook,
    state: &Arc<Mutex<TokenState>>,
    cancel_sig: &CancelSignal,
) {
    let total = state.lock().unwrap().total();
    if total > hook.threshold() {
        state.lock().unwrap().compaction_needed = true;
        cancel_sig.cancel();
    }
}

fn gemini_continuation_source() -> String {
    // CARGO_MANIFEST_DIR = codelet/cli
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/gemini_continuation.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ===========================================================================
// Scenario: Continuation TokenState reports the true total when the tracker
//           total is cache-inclusive
// ===========================================================================

#[test]
fn continuation_token_state_reports_true_total_not_double_counted() {
    // @step Given the token tracker was updated via update_display_only with 30k fresh input, 150k cache read and 2k cumulative output
    let tracker = tracker_after_display_update();

    // @step When the Gemini continuation TokenState is seeded from the tracker
    let state = TokenState::from_cache_inclusive_total(tracker.input_tokens, tracker.output_tokens);

    // @step Then the TokenState total is 182k
    assert_eq!(
        state.total(),
        182_000,
        "continuation seed must report the true context total exactly once"
    );

    // @step And the TokenState total is not 332k
    assert_ne!(
        state.total(),
        332_000,
        "continuation seed must never re-add cache on top of the cache-inclusive total"
    );
}

// ===========================================================================
// Scenario: CompactionHook does not trigger early during a Gemini continuation
// ===========================================================================

#[test]
fn compaction_hook_does_not_trigger_early_during_continuation() {
    // @step Given a continuation TokenState seeded from a 180k cache-inclusive tracker total with 2k output
    let tracker = tracker_after_display_update();
    let state = Arc::new(Mutex::new(TokenState::from_cache_inclusive_total(
        tracker.input_tokens,
        tracker.output_tokens,
    )));

    // @step And a compaction threshold of 200k that lies between the true total and the cache-inflated total
    let threshold = 200_000_u64;
    assert!(182_000 < threshold && threshold < 332_000, "sanity");
    let hook = CompactionHook::new(Arc::clone(&state), threshold);
    let cancel_sig = CancelSignal::new();

    // @step When the CompactionHook evaluates the threshold before the continuation call
    simulate_hook_check(&hook, &state, &cancel_sig);

    // @step Then compaction_needed remains false
    assert!(
        !state.lock().unwrap().compaction_needed,
        "hook must see the true 182k total, not the cache-inflated 332k, \
         and must NOT trigger compaction below the 200k threshold"
    );
}

// ===========================================================================
// Scenario: CompactionHook still triggers when the true total genuinely
//           exceeds the threshold
// ===========================================================================

#[test]
fn compaction_hook_still_triggers_on_genuine_overflow() {
    // @step Given a continuation TokenState seeded from a 180k cache-inclusive tracker total with 2k output
    let tracker = tracker_after_display_update();
    let state = Arc::new(Mutex::new(TokenState::from_cache_inclusive_total(
        tracker.input_tokens,
        tracker.output_tokens,
    )));

    // @step And a compaction threshold of 150k that is below the true total
    let threshold = 150_000_u64;
    let hook = CompactionHook::new(Arc::clone(&state), threshold);
    let cancel_sig = CancelSignal::new();

    // @step When the CompactionHook evaluates the threshold before the continuation call
    simulate_hook_check(&hook, &state, &cancel_sig);

    // @step Then compaction_needed is set to true
    assert!(
        state.lock().unwrap().compaction_needed,
        "the fix must not mask genuine overflows: 182k true total > 150k threshold"
    );
}

// ===========================================================================
// Scenario: Continuation TokenState construction site routes through the
//           audited constructor
// ===========================================================================

#[test]
fn continuation_seed_site_routes_through_audited_constructor() {
    // @step Given the gemini_continuation.rs source file
    let src = gemini_continuation_source();
    let normalized: String = src.split_whitespace().collect::<Vec<_>>().join("");

    // @step When the continuation TokenState construction at the tracker-basis seed site is inspected
    // (normalise whitespace so the multi-line struct/call shape is matched robustly)

    // @step Then the seed routes through TokenState from_cache_inclusive_total
    assert!(
        normalized
            .contains("TokenState::from_cache_inclusive_total(session.token_tracker.input_tokens"),
        "the tracker-basis continuation seed must route through the audited \
         TokenState::from_cache_inclusive_total constructor"
    );

    // @step And the cache-inflating seed shape with tracker total plus display cache fields is absent
    assert!(
        !normalized.contains(
            "input_tokens:session.token_tracker.input_tokens,\
             cache_read_input_tokens:current_display.cache_read_tokens"
        ),
        "the buggy seed shape (cache-inclusive tracker total alongside non-zero \
         display cache fields) must be gone from gemini_continuation.rs"
    );
}

// ===========================================================================
// Scenario: Nested continuation TokenState keeps its display-basis seeding
//           unchanged
// ===========================================================================

#[test]
fn nested_continuation_display_basis_seed_is_unchanged() {
    // @step Given a nested continuation TokenState seeded from a display snapshot with 30k raw input, 150k cache read and 2k output
    // (display-basis: raw input + SEPARATE cache fields — the cont_final
    // snapshot shape at gemini_continuation.rs:310-316, which is correct)
    let state = TokenState {
        input_tokens: 30_000,
        cache_read_input_tokens: 150_000,
        cache_creation_input_tokens: 0,
        output_tokens: 2_000,
        compaction_needed: false,
    };

    // @step When the TokenState total is computed
    let total = state.total();

    // @step Then the total is 182k with cache fields carried separately from raw input
    assert_eq!(
        total, 182_000,
        "display-basis seeding (raw + separate cache) already totals correctly"
    );
    assert_eq!(state.input_tokens, 30_000, "raw input stays raw");
    assert_eq!(
        state.cache_read_input_tokens, 150_000,
        "cache stays separate"
    );

    // Structural pin: the nested seed site must keep sourcing from the
    // display-basis cont_final snapshot, never from the tracker.
    let normalized: String = gemini_continuation_source()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("");
    assert!(
        normalized.contains(
            "input_tokens:cont_final.input_tokens,\
             cache_read_input_tokens:cont_final.cache_read_tokens"
        ),
        "the nested continuation TokenState must remain display-basis \
         (cont_final raw input + separate cache fields)"
    );
}
