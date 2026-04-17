#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/fix-cumulative-billed-output-never-incrementing-all-update-from-usage-call-sites-pass-output-tokens-0.feature
//!
//! TOKEN-001: fix cumulative_billed_output never incrementing — all
//! `update_from_usage` call sites in the interactive stream-loop module used
//! to hardcode `0` as the `output_tokens` argument to `ApiTokenUsage::new`,
//! which silently dropped every per-turn output from the cumulative billing
//! accumulator.
//!
//! These tests exercise the extracted helper
//! `flush_partial_state_before_compaction` (representative of the four call
//! sites) to verify that:
//!
//! 1. Successive flushes accumulate `cumulative_billed_output` correctly.
//! 2. A backwards tick in the cumulative display never underflows and never
//!    decreases the cumulative billing accumulator.
//! 3. `TokenTracker::compute_output_delta` is the single production-code
//!    source of the `saturating_sub(.., output_tokens)` pattern, and all four
//!    call sites delegate to it.

use codelet_cli::interactive::flush_partial_state_before_compaction;
use codelet_cli::session::Session;
use codelet_core::StreamingTokenDisplay;
use codelet_core::compaction::TokenTracker;

fn fresh_session() -> Session {
    Session::new(None).expect("failed to create test session")
}

/// Build a `StreamingTokenDisplay` whose `current().output_tokens` equals
/// `cumulative_output` by seeding the `prev_output` slot of the constructor.
/// Input side carries the caller-supplied `input_tokens` through
/// `cache_read` so `total_input()` is non-trivial, matching real streams.
fn display_with_cumulative_output(cumulative_output: u64, total_input: u64) -> StreamingTokenDisplay {
    StreamingTokenDisplay::new(total_input, cumulative_output, 0, 0)
}

// ============================================================================
// Scenario: Cumulative billed output accumulates correctly across two sequential turns
// ============================================================================

#[test]
fn cumulative_billed_output_accumulates_across_two_sequential_turns() {
    // @step Given a fresh session where session.token_tracker.output_tokens equals 0 and cumulative_billed_output equals 0
    let mut session = fresh_session();
    session.token_tracker = TokenTracker::default();
    assert_eq!(session.token_tracker.output_tokens, 0);
    assert_eq!(session.token_tracker.cumulative_billed_output, 0);

    // @step When turn one completes and reports a cumulative output of 100 tokens
    let turn1_display = display_with_cumulative_output(100, 500);
    let mut turn1_text = String::new();
    flush_partial_state_before_compaction(&mut session, &mut turn1_text, &turn1_display)
        .expect("turn 1 flush must succeed");

    // @step And turn two completes and reports a cumulative output of 220 tokens
    let turn2_display = display_with_cumulative_output(220, 500);
    let mut turn2_text = String::new();
    flush_partial_state_before_compaction(&mut session, &mut turn2_text, &turn2_display)
        .expect("turn 2 flush must succeed");

    // @step Then session.token_tracker.cumulative_billed_output should equal 220
    assert_eq!(
        session.token_tracker.cumulative_billed_output, 220,
        "cumulative billed output must accumulate per-turn deltas (100 + 120 = 220)"
    );

    // @step And session.token_tracker.output_tokens should equal 220
    assert_eq!(
        session.token_tracker.output_tokens, 220,
        "output_tokens display must reflect the latest cumulative value"
    );
}

// ============================================================================
// Scenario: Per-turn delta never underflows when the cumulative display ticks backward
// ============================================================================

#[test]
fn per_turn_delta_never_underflows_when_cumulative_ticks_backward() {
    // @step Given a session where session.token_tracker.output_tokens equals 500 and cumulative_billed_output equals 500
    let mut session = fresh_session();
    session.token_tracker = TokenTracker {
        output_tokens: 500,
        cumulative_billed_output: 500,
        ..Default::default()
    };

    // @step When the stream reports a cumulative output of 300 tokens
    let ticked_back_display = display_with_cumulative_output(300, 500);
    let mut text = String::new();
    flush_partial_state_before_compaction(&mut session, &mut text, &ticked_back_display)
        .expect("flush must succeed even on backward tick");

    // @step Then session.token_tracker.cumulative_billed_output should remain 500
    assert_eq!(
        session.token_tracker.cumulative_billed_output, 500,
        "cumulative billed output must not decrease on a backward tick"
    );

    // @step And session.token_tracker.output_tokens should equal the reported cumulative
    assert_eq!(
        session.token_tracker.output_tokens, 300,
        "output_tokens should directly reflect the reported cumulative without clamping"
    );

    // @step And no panic occurs from integer underflow
    // (implicit — if we reached this line, no panic occurred)
}

// ============================================================================
// Scenario: Delta computation is centralized in a single helper
// ============================================================================

#[test]
fn delta_computation_is_centralized_in_single_helper() {
    // @step Given the fspec codelet-cli Rust crate
    // Read the four interactive-module source files directly from the repo so
    // this test fails if any future refactor re-introduces a duplicate
    // `saturating_sub(.. , output_tokens)` outside the helper.
    let stream_loop = include_str!("../src/interactive/stream_loop.rs");
    let gemini_continuation = include_str!("../src/interactive/gemini_continuation.rs");
    let recovery_compaction = include_str!("../src/interactive/recovery_compaction.rs");
    // The helper itself lives in codelet-core. It is the single production-code
    // site allowed to perform the saturating subtraction on an output_tokens
    // field.
    let token_tracker_model = include_str!("../../core/src/compaction/model.rs");

    // @step When I search the interactive module for `saturating_sub` applied to `.output_tokens` fields
    fn count_output_saturating_sub(src: &str) -> usize {
        // Count every occurrence of `saturating_sub(<something that contains
        // output_tokens>)` — this captures both orderings:
        //   foo.output_tokens.saturating_sub(bar)
        //   bar.saturating_sub(self.output_tokens)
        //   saturating_sub(some.output_tokens)
        //
        // The canonical helper in `TokenTracker::compute_output_delta` uses
        // the second form: `current_cumulative_output.saturating_sub(self.output_tokens)`.
        // That is the single production-code site allowed across the crate.
        let mut count = 0;
        let needle = ".saturating_sub(";
        for (idx, _) in src.match_indices(needle) {
            let after = &src[idx + needle.len()..];
            // Look for the matching close-paren to delimit the argument.
            let mut depth: i32 = 1;
            let mut end = 0;
            for (i, ch) in after.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let arg = &after[..end];
            // Also inspect the 40 characters preceding the `.saturating_sub(`
            // so receiver-form usage (`x.output_tokens.saturating_sub(...)`)
            // is counted too.
            let prefix_start = idx.saturating_sub(40);
            let prefix = &src[prefix_start..idx];
            if arg.contains("output_tokens") || prefix.contains("output_tokens") {
                count += 1;
            }
        }
        count
    }

    let interactive_count = count_output_saturating_sub(stream_loop)
        + count_output_saturating_sub(gemini_continuation)
        + count_output_saturating_sub(recovery_compaction);
    let helper_count = count_output_saturating_sub(token_tracker_model);

    // @step Then the only production-code match is inside the single TokenTracker helper method
    assert_eq!(
        interactive_count, 0,
        "no interactive-module file may call saturating_sub on an output_tokens \
         field directly — all four call sites must delegate to \
         TokenTracker::compute_output_delta"
    );
    assert_eq!(
        helper_count, 1,
        "exactly one production-code saturating_sub on an output_tokens field \
         is allowed, and it must live inside TokenTracker::compute_output_delta"
    );

    // @step And all four update_from_usage call sites delegate to that helper
    // Each of the four call sites must mention `compute_output_delta` exactly
    // where it previously hardcoded a literal `0` in the `ApiTokenUsage::new`
    // call. We count occurrences across the three interactive files; the
    // update_token_tracker helper + the normal-completion branch in
    // gemini_continuation.rs account for two of the four.
    let delta_call_count = stream_loop.matches("compute_output_delta(").count()
        + gemini_continuation.matches("compute_output_delta(").count()
        + recovery_compaction.matches("compute_output_delta(").count();
    assert_eq!(
        delta_call_count, 4,
        "each of the four update_from_usage call sites must call \
         TokenTracker::compute_output_delta exactly once"
    );

    // Behavioral sanity check: the helper produces the arithmetically-correct
    // delta and saturates at zero on a backward tick.
    let tracker = TokenTracker {
        output_tokens: 100,
        ..Default::default()
    };
    assert_eq!(tracker.compute_output_delta(250), 150);
    assert_eq!(tracker.compute_output_delta(80), 0);
}
