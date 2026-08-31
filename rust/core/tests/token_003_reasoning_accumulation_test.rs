//! Feature: spec/features/sessionheader-reasoning-tokens-do-not-accumulate-each-api-usage-overwrites-the-last-value.feature
//!
//! TOKEN-003: The SessionHeader reasoning (🧠) counter must be
//! SESSION-CUMULATIVE — it accumulates across API segments within a turn
//! (tool loops) and across turns, matching the output (↑) counter semantics.
//!
//! These tests exercise `codelet-core` primitives:
//!
//! - `StreamingTokenDisplay` must track reasoning with the same cumulative
//!   structure as output (cumulative base + current-segment value), seed the
//!   previous session's cumulative via `with_prev_reasoning(u64)`, and report
//!   `base + current_segment` from `current()` / `update_from_*`.
//! - `TokenTracker` must store the session-cumulative reasoning value passed
//!   in via `ApiTokenUsage::with_reasoning_tokens`, and keep it across
//!   `reset_after_compaction()` (session-spend metric, per Assumption 1).
//! - Compaction threshold / fill-percentage math must use physical
//!   per-segment context only, never the cumulative reasoning display value.
//!
//! Persistence (Scenario 3) is covered in the persistence section below:
//! `persist_token_state` gains a `reasoning_tokens: u32` parameter and writes
//! it into the manifest `TokenUsage` struct (which gains a
//! `reasoning_tokens` field); `set_session_tokens` (the /resume restore
//! path) gains a `reasoning_tokens: u64` parameter and writes it back.
//! Those behavioural tests serialise via `serial_test::serial` because they
//! reach for the process-global data directory + SESSION_STORE singleton
//! (same pattern as `rpc080_agent_loop_persistence.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use codelet_agent_loop::persist::persist_token_state;
use codelet_core::compaction::TokenTracker;
use codelet_core::persistence::{
    create_session, load_session, reset_stores_for_tests, set_session_tokens,
};
use codelet_core::streaming_display::StreamingTokenDisplay;
use codelet_core::ApiTokenUsage;
use rig::completion::Usage;
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

/// Build a rig `Usage` with an explicit reasoning value (None = provider
/// reports no reasoning tokens).
fn make_usage(input: u64, output: u64, reasoning: Option<u64>) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: input + output,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        reasoning_tokens: reasoning,
    }
}

// ============================================================================
// Scenario: Reasoning tokens accumulate across API segments within a single turn
// ============================================================================

#[test]
fn reasoning_tokens_accumulate_across_api_segments_within_a_single_turn() {
    // @step Given a session using a provider that reports reasoning tokens
    // Fresh session: no previous cumulative reasoning to seed.
    let mut display = StreamingTokenDisplay::new(1_000, 0, 0, 0);

    // @step And a turn with two API segments where segment 1 reports 500 reasoning tokens and segment 2 reports 300 reasoning tokens
    // Segment 1: authoritative Usage event reports 500 reasoning tokens.
    let segment1_usage = make_usage(1_500, 95, Some(500));
    let update1 = display.update_from_usage(&segment1_usage).unwrap();
    assert_eq!(
        update1.reasoning_tokens, 500,
        "segment 1 display must show its own 500 reasoning tokens"
    );

    // Tool call starts segment 2 (MessageStart: output == 0).
    let segment2_start = make_usage(1_600, 0, None);
    display.start_new_segment(&segment2_start);

    // Segment 2: authoritative Usage event reports 300 reasoning tokens.
    let segment2_usage = make_usage(2_000, 75, Some(300));
    let update2 = display.update_from_usage(&segment2_usage).unwrap();

    // @step When the turn completes
    // @step Then the session reasoning total is 800
    assert_eq!(
        update2.reasoning_tokens, 800,
        "reasoning must accumulate across segments (500 + 300 = 800), \
         not be overwritten by the latest segment's 300"
    );
    assert_eq!(
        display.current().reasoning_tokens, 800,
        "current() must report the session-cumulative reasoning value"
    );

    // @step And the SessionHeader displays "800🧠" (not "300🧠")
    // The TUI renders the 🧠 suffix from TokenDisplayUpdate.reasoning_tokens,
    // so the emitted update must carry 800 — feeding the end-of-turn
    // session tracker from the same value keeps the header in sync.
    let mut tracker = TokenTracker::default();
    let end_of_turn_usage = ApiTokenUsage::new(2_000, 0, 0, 75)
        .with_reasoning_tokens(update2.reasoning_tokens);
    tracker.update_from_usage(&end_of_turn_usage, update2.output_tokens);
    assert_eq!(
        tracker.reasoning_tokens, 800,
        "session tracker must hold the session-cumulative reasoning total"
    );
}

// ============================================================================
// Scenario: Reasoning tokens accumulate across turns
// ============================================================================

#[test]
fn reasoning_tokens_accumulate_across_turns() {
    // @step Given a session where turn 1 used 800 reasoning tokens
    // Turn 2's display is seeded from session.token_tracker.reasoning_tokens
    // (the 6 stream_loop.rs seed sites + 2 gemini_continuation.rs sites).
    let mut display = StreamingTokenDisplay::new(1_000, 0, 0, 0).with_prev_reasoning(800);
    assert_eq!(
        display.current().reasoning_tokens, 800,
        "before turn 2 reports anything, the display must continue from the \
         previous session's cumulative 800"
    );

    // @step When turn 2 completes and uses 200 reasoning tokens
    let turn2_usage = make_usage(1_500, 50, Some(200));
    let update = display.update_from_usage(&turn2_usage).unwrap();

    // @step Then the SessionHeader displays "1000🧠"
    assert_eq!(
        update.reasoning_tokens, 1_000,
        "turn 2 must accumulate on top of turn 1 (800 + 200 = 1000), \
         not replace it with 200"
    );

    // @step And the previous turn's value is kept and accumulated, not replaced
    let mut tracker = TokenTracker::default();
    let end_of_turn_usage =
        ApiTokenUsage::new(1_500, 0, 0, 50).with_reasoning_tokens(update.reasoning_tokens);
    tracker.update_from_usage(&end_of_turn_usage, update.output_tokens);
    assert_eq!(
        tracker.reasoning_tokens, 1_000,
        "session tracker must hold 1000 after turn 2, not 200"
    );
}

// ============================================================================
// Scenario: No reasoning suffix when the provider reports no reasoning tokens
// ============================================================================

#[test]
fn no_reasoning_suffix_when_provider_reports_no_reasoning_tokens() {
    // @step Given a session using a provider that reports no reasoning tokens
    let mut display = StreamingTokenDisplay::new(1_000, 0, 0, 0);

    // @step When turns complete with zero reasoning tokens
    // Turn 1: no reasoning in the Usage event.
    let turn1_usage = make_usage(1_500, 50, None);
    let update1 = display.update_from_usage(&turn1_usage).unwrap();
    assert_eq!(
        update1.reasoning_tokens, 0,
        "no reasoning reported → counter stays 0 (TUI omits the 🧠 suffix)"
    );

    // Segment boundary must not fabricate reasoning either.
    let segment_start = make_usage(1_600, 0, None);
    display.start_new_segment(&segment_start);

    // Turn 2 (new segment): still no reasoning.
    let turn2_usage = make_usage(1_700, 40, None);
    let update2 = display.update_from_usage(&turn2_usage).unwrap();

    // @step Then the SessionHeader displays no 🧠 suffix
    // @step And the counter stays 0, exactly as today
    assert_eq!(
        update2.reasoning_tokens, 0,
        "counter must remain 0 across segments and turns when the provider \
         never reports reasoning tokens"
    );
    assert_eq!(display.current().reasoning_tokens, 0);
}

// ============================================================================
// Scenario: Reasoning counter never ticks backward within a session
// ============================================================================

#[test]
fn reasoning_counter_never_ticks_backward_within_a_session() {
    // @step Given a session with 1000 cumulative reasoning tokens
    let mut display = StreamingTokenDisplay::new(1_000, 0, 0, 0).with_prev_reasoning(1_000);
    let baseline = display.current().reasoning_tokens;
    assert_eq!(baseline, 1_000);

    // @step When a new turn reports a lower per-segment reasoning value
    let lower_segment_usage = make_usage(1_500, 50, Some(300));
    let update = display.update_from_usage(&lower_segment_usage).unwrap();

    // @step Then the displayed reasoning total is still at least 1000
    assert!(
        update.reasoning_tokens >= 1_000,
        "a lower per-segment report (300) must not drag the session total \
         below the previous cumulative 1000; got {}",
        update.reasoning_tokens
    );

    // @step And the counter never decreases
    // A later usage event within the same segment reporting an even lower
    // value must not decrease the display either.
    let even_lower_usage = make_usage(1_600, 60, Some(100));
    let update2 = display.update_from_usage(&even_lower_usage).unwrap();
    assert!(
        update2.reasoning_tokens >= update.reasoning_tokens,
        "the reasoning display must be monotonically non-decreasing within a \
         session ({} → {})",
        update.reasoning_tokens,
        update2.reasoning_tokens
    );
    assert!(
        display.current().reasoning_tokens >= baseline,
        "current() must never report less than the seeded cumulative"
    );
}

// ============================================================================
// Scenario: Cumulative reasoning does not affect compaction threshold math
// ============================================================================

#[test]
fn cumulative_reasoning_does_not_affect_compaction_threshold_math() {
    // @step Given a session with a large cumulative reasoning total but small current context occupancy
    let tracker = TokenTracker {
        input_tokens: 1_000,
        output_tokens: 75,
        reasoning_tokens: 100_000, // session-cumulative display value
        ..Default::default()
    };

    // @step When the context fill percentage is computed for the [X%] badge
    // The fill badge is driven by per-request physical usage
    // (`emit_context_fill_from_usage` consumes `ApiTokenUsage::total_context`),
    // never by the session tracker's cumulative reasoning display value.
    let per_segment_usage =
        ApiTokenUsage::new(1_000, 0, 0, 75).with_reasoning_tokens(300);
    let fill_basis = per_segment_usage.total_context();

    // @step Then the threshold check uses physical context occupancy (input + current-segment output + current-segment reasoning)
    assert_eq!(
        fill_basis,
        1_000 + 75 + 300,
        "fill basis must be physical per-segment occupancy only"
    );

    // @step And the cumulative reasoning display value does not inflate the fill percentage
    assert_eq!(
        tracker.effective_tokens(),
        1_000,
        "effective context (fill/threshold basis) must ignore the \
         100k cumulative reasoning display value"
    );
}

// ============================================================================
// Assumption 1: reset_after_compaction keeps the cumulative reasoning value
// (session-spend metric, like cumulative_billed_*)
// ============================================================================

#[test]
fn reset_after_compaction_keeps_cumulative_reasoning() {
    let mut tracker = TokenTracker {
        input_tokens: 100_000,
        output_tokens: 500,
        reasoning_tokens: 1_000,
        cumulative_billed_input: 200_000,
        cumulative_billed_output: 500,
        ..Default::default()
    };

    tracker.reset_after_compaction();

    assert_eq!(
        tracker.reasoning_tokens, 1_000,
        "cumulative reasoning is a session-spend metric and must survive \
         compaction resets (like cumulative_billed_*)"
    );
    // Context metrics are still reset.
    assert_eq!(tracker.output_tokens, 0);
    assert_eq!(tracker.cumulative_billed_input, 200_000);
}

// ============================================================================
// Supporting: TokenTracker stores the session-cumulative reasoning value
// passed in via with_reasoning_tokens (end-of-turn update contract)
// ============================================================================

#[test]
fn token_tracker_update_from_usage_stores_cumulative_reasoning() {
    let mut tracker = TokenTracker::default();

    // The end-of-turn call sites must pass the turn's CUMULATIVE reasoning
    // value (from StreamingTokenDisplay.current()) via with_reasoning_tokens.
    let usage =
        ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000).with_reasoning_tokens(1_000);
    tracker.update_from_usage(&usage, 220);

    assert_eq!(
        tracker.reasoning_tokens, 1_000,
        "tracker must hold the session-cumulative reasoning value supplied by \
         the caller, not zero it out"
    );
    // total_tokens still includes reasoning for display purposes.
    assert_eq!(
        tracker.total_tokens(),
        tracker.input_tokens + tracker.output_tokens + tracker.reasoning_tokens
    );
}

// ============================================================================
// Persistence helpers (Scenario 3) — same hermetic pattern as
// rpc080_agent_loop_persistence.rs
// ============================================================================

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

/// Create a hermetic session and return (session_id, _data_dir_guard).
fn fresh_session(name: &str) -> (Uuid, TempDir) {
    let guard = setup_data_dir();
    let project = PathBuf::from("/test/project/token003");
    let session = create_session(name, &project).expect("create_session");
    (session.id, guard)
}

// ============================================================================
// Scenario: Reasoning tokens persist across session restore
// ============================================================================

#[test]
#[serial]
fn reasoning_tokens_persist_across_session_restore() {
    // @step Given a session that accumulated 1000 reasoning tokens across several turns
    let (session_id, _guard) = fresh_session("reasoning-persist");

    // @step When the session is closed and restored via /resume
    // Closing: the end-of-turn persist path writes the session-cumulative
    // reasoning value into the manifest.
    persist_token_state(&session_id, 100, 50, 1_000)
        .expect("persist_token_state must succeed");

    // Restoring: /resume re-seeds the manifest token state via
    // set_session_tokens, which must carry the reasoning value through.
    let mut manifest = load_session(session_id).expect("reload manifest for restore");
    set_session_tokens(
        &mut manifest,
        &codelet_core::persistence::TokenUsage {
            current_context_tokens: 100,
            cumulative_billed_input: 100,
            cumulative_billed_output: 50,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            reasoning_tokens: 1_000, // reasoning_tokens (session cumulative)
        },
    )
    .expect("set_session_tokens must succeed");

    // @step Then the SessionHeader displays "1000🧠" again
    // The SessionHeader seeds its reasoning counter from the restored
    // manifest token state — it must read back 1000.
    let restored = load_session(session_id).expect("reload manifest after restore");
    assert_eq!(
        restored.token_usage.reasoning_tokens, 1_000,
        "restored manifest must carry the session-cumulative reasoning value \
         so the SessionHeader re-displays 1000🧠"
    );

    // @step And the reasoning value was persisted to the session manifest
    // The persist path (before restore) must have written the value too.
    let after_persist = load_session(session_id).expect("reload manifest after persist");
    assert_eq!(
        after_persist.token_usage.reasoning_tokens, 1_000,
        "persist_token_state must write reasoning_tokens into the manifest \
         TokenUsage struct"
    );
}

// ============================================================================
// Supporting: persist_token_state with zero reasoning keeps the manifest at 0
// (non-thinking providers must not fabricate a 🧠 value on restore)
// ============================================================================

#[test]
#[serial]
fn persist_token_state_zero_reasoning_stays_zero() {
    // @step Given a session using a provider that reports no reasoning tokens
    let (session_id, _guard) = fresh_session("reasoning-zero");

    // @step When turns complete with zero reasoning tokens
    persist_token_state(&session_id, 100, 50, 0).expect("persist_token_state must succeed");

    // @step Then the SessionHeader displays no 🧠 suffix
    // @step And the counter stays 0, exactly as today
    let manifest = load_session(session_id).expect("reload manifest");
    assert_eq!(
        manifest.token_usage.reasoning_tokens, 0,
        "zero reasoning must persist as zero — the TUI omits the 🧠 suffix \
         when the restored value is 0"
    );
}

// ============================================================================
// Source-shape: manifest TokenUsage struct has a reasoning_tokens field
// ============================================================================

#[test]
fn manifest_token_usage_has_reasoning_tokens_field() {
    // @step Given the manifest TokenUsage struct in rust/core/src/persistence/manifest.rs
    let manifest_src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/persistence/manifest.rs"),
    )
    .unwrap_or_else(|e| panic!("must read manifest.rs: {e}"));

    // @step When the TokenUsage struct definition is scanned
    let struct_start = manifest_src
        .find("pub struct TokenUsage")
        .expect("TokenUsage struct must exist in manifest.rs");
    let struct_body = &manifest_src[struct_start..];
    let closing_brace = struct_body
        .find('}')
        .expect("TokenUsage struct must be closed");
    let body = &struct_body[..closing_brace];

    // @step Then it declares a reasoning_tokens field
    assert!(
        body.contains("reasoning_tokens"),
        "TokenUsage must declare a reasoning_tokens field so the \
         session-cumulative reasoning value can be persisted"
    );
}

// ============================================================================
// Source-shape: agent-loop persist_token_state signature carries reasoning
// ============================================================================

#[test]
fn persist_token_state_signature_carries_reasoning() {
    // @step Given the source of rust/agent-loop/src/persist.rs
    let persist_src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../agent-loop/src/persist.rs"),
    )
    .unwrap_or_else(|e| panic!("must read persist.rs: {e}"));

    // @step When the persist_token_state function signature is scanned
    let fn_start = persist_src
        .find("pub fn persist_token_state")
        .expect("persist_token_state must exist");
    let signature_end = persist_src[fn_start..]
        .find(')')
        .expect("persist_token_state signature must be closed");
    let signature = &persist_src[fn_start..fn_start + signature_end];

    // @step Then the signature accepts a reasoning_tokens parameter
    assert!(
        signature.contains("reasoning_tokens"),
        "persist_token_state must accept a reasoning_tokens parameter so the \
         session-cumulative reasoning value reaches the manifest"
    );
}
