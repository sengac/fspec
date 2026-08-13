#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/token-accounting-cache-integrity.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! CMPCT-041 root fix: at every turn-start seed/re-seed site the
//! StreamingTokenDisplay must be seeded through the single audited
//! `from_cache_inclusive_total` constructor which de-overlaps the
//! cache-inclusive tracker total (raw = total - cache) instead of storing the
//! total in the raw slot alongside non-zero cache fields. Without the fix the
//! seed emit re-adds cache via `TokenDisplayUpdate::total_input()` and a true
//! 180k context (30k raw + 150k cache_read) is emitted as 330k.

use std::fs;
use std::path::PathBuf;

use codelet_cli::interactive::flush_partial_state_before_compaction;
use codelet_cli::interactive::output::TokenInfo;
use codelet_cli::session::Session;
use codelet_core::{ApiTokenUsage, StreamingTokenDisplay};

// ===========================================================================
// Helpers
// ===========================================================================

fn fresh_session() -> Session {
    // Mirror cmpct038_measurement_basis_test: in multi-credential
    // environments ProviderManager::new() refuses to auto-select, so fall
    // back to an explicit provider.
    let provider_manager = codelet_providers::ProviderManager::new()
        .or_else(|_| codelet_providers::ProviderManager::with_provider("gemini"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("zai"))
        .or_else(|_| codelet_providers::ProviderManager::with_provider("claude"))
        .expect("Need at least one API key for tests");
    Session::from_provider_manager(provider_manager)
}

/// Drive the tracker through a completed turn N-1 with the worked example
/// from the CMPCT-041 dossier: 30k raw + 150k cache_read = 180k total.
/// After `update_from_usage` the tracker stores the CACHE-INCLUSIVE total
/// (PROV-001) alongside the non-zero cache fields.
fn complete_prior_turn(session: &mut Session) {
    let usage = ApiTokenUsage::new(30_000, 150_000, 0, 500);
    session.token_tracker.update_from_usage(&usage, 500);
    assert_eq!(
        session.token_tracker.input_tokens, 180_000,
        "sanity: tracker stores the cache-inclusive total"
    );
    assert_eq!(
        session.token_tracker.cache_read_input_tokens,
        Some(150_000),
        "sanity: tracker stores cache_read alongside the total"
    );
}

/// Seed a display exactly the way the post-fix turn-start sites do: from the
/// tracker's cache-inclusive total plus its cache fields.
fn seed_from_tracker(session: &Session) -> StreamingTokenDisplay {
    StreamingTokenDisplay::from_cache_inclusive_total(
        session.token_tracker.input_tokens,
        session.token_tracker.output_tokens,
        session.token_tracker.cache_read_input_tokens.unwrap_or(0),
        session
            .token_tracker
            .cache_creation_input_tokens
            .unwrap_or(0),
    )
}

fn stream_loop_source() -> String {
    // CARGO_MANIFEST_DIR = rust/cli
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/stream_loop.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ===========================================================================
// Scenario: Turn-start seed emit reports the true cache-inclusive total
//           exactly once
// ===========================================================================

#[test]
fn seed_emit_reports_true_total_not_double_counted() {
    // @step Given a completed turn left the token tracker at a cache-inclusive total of 180000 tokens with 150000 cache-read and 0 cache-creation tokens
    let mut session = fresh_session();
    complete_prior_turn(&mut session);

    // @step When the next turn seeds the streaming token display from the tracker and emits the initial token state
    let display = seed_from_tracker(&session);
    let emitted: TokenInfo = display.current().into();

    // @step Then the emitted token update reports 180000 input tokens
    assert_eq!(
        emitted.input_tokens, 180_000,
        "seed emit must report the true cache-inclusive total exactly once"
    );

    // @step And the emitted token update never reports the double-counted 330000 tokens
    assert_ne!(
        emitted.input_tokens, 330_000,
        "seed emit must never re-add cache on top of the cache-inclusive total"
    );
}

/// SUPPLEMENTARY STRUCTURAL GUARD — NOT a Gherkin scenario test.
///
/// The behavioral test above (`seed_emit_reports_true_total_not_double_counted`)
/// carries the @step coverage for the "Turn-start seed emit reports the true
/// cache-inclusive total exactly once" scenario. This guard has no @step
/// comments on purpose: it pins the wiring shape so no seed site can regress:
///
/// 1. Every tracker-basis seed/re-seed site in stream_loop.rs must route
///    through the audited `from_cache_inclusive_total` constructor so no
///    site can reintroduce the double-count by passing tracker cache fields
///    to `StreamingTokenDisplay::new` alongside the cache-inclusive total.
/// 2. The two display-basis seeds in gemini_continuation.rs are the ONLY
///    permitted `StreamingTokenDisplay::new` uses in the interactive stream
///    path: they re-seed from an existing display snapshot
///    (`current_display.*` / `cont_final.*` — raw input + separate cache
///    fields) and MUST NOT be fed tracker cache-inclusive totals.
#[test]
fn stream_loop_seed_sites_route_through_audited_constructor() {
    let src = stream_loop_source();

    let raw_new_calls = src.matches("StreamingTokenDisplay::new(").count();
    let audited_calls = src
        .matches("StreamingTokenDisplay::from_cache_inclusive_total(")
        .count();

    assert_eq!(
        raw_new_calls, 0,
        "stream_loop.rs must not seed displays via StreamingTokenDisplay::new; \
         found {raw_new_calls} raw call(s) — every seed/re-seed site must use \
         from_cache_inclusive_total"
    );

    assert!(
        audited_calls >= 5,
        "stream_loop.rs must route all five seed/re-seed sites (turn start, \
         post-compaction restart, thinking-exhaustion retry, truncation retry, \
         network retry) through from_cache_inclusive_total; found {audited_calls}"
    );

    // CMPCT-041 review W2: pin the two display-basis re-seeds in
    // gemini_continuation.rs. Normalise whitespace so the multi-line call
    // shape is matched robustly, then assert each `::new` sources from a
    // display snapshot's raw `input_tokens` (display basis), never from
    // `token_tracker` (cache-inclusive basis).
    let gemini_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/gemini_continuation.rs");
    let gemini_src = fs::read_to_string(&gemini_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", gemini_path.display(), e));
    let normalized: String = gemini_src.split_whitespace().collect::<Vec<_>>().join("");

    let gemini_new_calls = gemini_src.matches("StreamingTokenDisplay::new(").count();
    assert_eq!(
        gemini_new_calls, 2,
        "gemini_continuation.rs must contain exactly its two display-basis \
         re-seed sites; found {gemini_new_calls}"
    );
    assert!(
        normalized.contains("StreamingTokenDisplay::new(current_display.input_tokens,"),
        "the continuation re-seed must source from the display-basis snapshot \
         current_display (raw input + separate cache fields)"
    );
    assert!(
        normalized.contains("StreamingTokenDisplay::new(cont_final.input_tokens,"),
        "the nested-continuation re-seed must source from the display-basis \
         snapshot cont_final (raw input + separate cache fields)"
    );
    assert!(
        !normalized.contains("StreamingTokenDisplay::new(session.token_tracker"),
        "gemini_continuation.rs must never feed tracker cache-inclusive totals \
         into StreamingTokenDisplay::new"
    );
}

// ===========================================================================
// Scenario: Pre-compaction flush in the no-usage-event window preserves
//           tracker and billing integrity
// ===========================================================================

#[test]
fn flush_in_no_usage_window_preserves_tracker_and_billing() {
    // @step Given the streaming token display was seeded from a tracker total of 180000 tokens including 150000 cache-read tokens
    let mut session = fresh_session();
    complete_prior_turn(&mut session);
    let billed_input_before_turn = session.token_tracker.cumulative_billed_input;
    let display = seed_from_tracker(&session);

    // @step And no Usage event has arrived this turn because the prompt was rejected as too long
    // (the display still holds only the seeded values — no update_from_usage /
    // update_from_final_response calls this turn)
    let mut assistant_text = String::new();

    // @step When partial state is flushed before compaction recovery
    flush_partial_state_before_compaction(&mut session, &mut assistant_text, &display)
        .expect("flush must not fail");

    // @step Then the token tracker still reads 180000 input tokens
    assert_eq!(
        session.token_tracker.input_tokens, 180_000,
        "flush must not corrupt the tracker to the double-counted 330k value"
    );

    // @step And cumulative billed input grows by only the 30000 raw input tokens
    let billed_delta = session.token_tracker.cumulative_billed_input - billed_input_before_turn;
    assert_eq!(
        billed_delta, 30_000,
        "flush must bill only raw input; cache tokens must not be absorbed \
         into cumulative_billed_input (got delta {billed_delta})"
    );
}

// ===========================================================================
// Scenario: Inconsistent stale cache split falls back to the trusted total
// ===========================================================================

#[test]
fn stale_cache_larger_than_total_falls_back_to_trusted_total() {
    // @step Given a freshly recalculated post-compaction tracker total of 5000 tokens alongside stale cache-read values of 150000 tokens
    let total = 5_000_u64;
    let stale_cache_read = 150_000_u64;

    // @step When the streaming token display is seeded from those tracker values
    let display = StreamingTokenDisplay::from_cache_inclusive_total(total, 0, stale_cache_read, 0);
    let emitted: TokenInfo = display.current().into();

    // @step Then the emitted token update reports 5000 input tokens
    assert_eq!(
        emitted.input_tokens, 5_000,
        "seeding must trust the recalculated total when the cache split is inconsistent"
    );

    // @step And the stale cache split is dropped instead of inflating the total
    let update = display.current();
    assert_eq!(
        update.cache_read_tokens, 0,
        "inconsistent stale cache_read must be dropped, not carried into the display"
    );
    assert_eq!(
        update.cache_creation_tokens, 0,
        "inconsistent stale cache_creation must be dropped, not carried into the display"
    );
}

// ===========================================================================
// Scenario: Mid-stream Usage self-heal behavior is unchanged
// ===========================================================================

#[test]
fn mid_stream_usage_self_heal_is_unchanged() {
    // @step Given a streaming token display seeded from a cache-inclusive tracker total of 180000 tokens
    let mut display = StreamingTokenDisplay::from_cache_inclusive_total(180_000, 500, 150_000, 0);
    let seeded: TokenInfo = display.current().into();
    assert_eq!(
        seeded.input_tokens, 180_000,
        "sanity: seed emits true total"
    );

    // @step When an authoritative mid-stream Usage event arrives with 35000 raw input, 160000 cache-read, and 0 cache-creation tokens
    let usage = rig::completion::Usage {
        input_tokens: 35_000,
        output_tokens: 600,
        total_tokens: 35_600,
        cache_read_input_tokens: Some(160_000),
        cache_creation_input_tokens: Some(0),
        reasoning_tokens: None,
    };
    let update = display
        .update_from_usage(&usage)
        .expect("Usage events always emit");

    // @step Then the display reports the authoritative 195000-token total
    assert_eq!(
        update.total_input(),
        195_000,
        "mid-stream Usage self-heal must keep producing authoritative totals"
    );
    let healed: TokenInfo = update.into();
    assert_eq!(healed.input_tokens, 195_000);

    // @step And subsequent emits use the authoritative raw and cache values from the Usage event
    let current = display.current();
    assert_eq!(current.input_tokens, 35_000, "raw input from Usage");
    assert_eq!(current.cache_read_tokens, 160_000, "cache_read from Usage");
    assert_eq!(
        current.cache_creation_tokens, 0,
        "cache_creation from Usage"
    );
}
