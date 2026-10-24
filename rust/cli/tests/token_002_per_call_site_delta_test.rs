#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/fix-cumulative-billed-output-never-incrementing-all-update-from-usage-call-sites-pass-output-tokens-0.feature
//!
//! TOKEN-002: per-call-site integration tests for `compute_output_delta`.
//!
//! Delta-based usage updates in the main stream and compaction recovery must
//! pass `TokenTracker::compute_output_delta` results into `ApiTokenUsage::new`.
//! Gemini normal completions return to the shared owner; helper exits update
//! display values and restore already-settled physical-request billing.
//! These source guards complement the real streamed BUG-223 integration tests.

const STREAM_LOOP_SRC: &str = include_str!("../src/interactive/stream_loop.rs");
#[path = "support/gemini_continuation_source.rs"]
mod gemini_source;
const RECOVERY_COMPACTION_SRC: &str = include_str!("../src/interactive/recovery_compaction.rs");

/// Return the byte offset of the `n`-th occurrence (1-indexed) of `needle`
/// in `haystack`, or `None` if fewer than `n` occurrences exist.
fn nth_occurrence(haystack: &str, needle: &str, n: usize) -> Option<usize> {
    haystack.match_indices(needle).nth(n - 1).map(|(i, _)| i)
}

/// Grab the `let <NAME> = session.token_tracker.compute_output_delta(<ARG>);`
/// fragment starting at `start`. Returns (binding_identifier, argument)
/// if the call site matches the expected shape.
fn parse_delta_binding(src: &str, start: usize) -> Option<(String, String)> {
    // Find the line start for the compute_output_delta call (may be split
    // across lines in the source).
    let snippet = &src[start..];
    // Walk backwards from `start` up to 512 bytes to find the last
    // `let` keyword on the same logical statement.
    let lookbehind_start = start.saturating_sub(512);
    let lookbehind = &src[lookbehind_start..start];
    let let_pos = lookbehind.rfind("let ")?;
    let binding_window = &lookbehind[let_pos + 4..];
    // Binding is everything up to the first `=`.
    let eq_pos = binding_window.find('=')?;
    let binding = binding_window[..eq_pos].trim().to_string();

    // Argument is what's between the opening `(` of the call and the
    // matching close-paren.
    let open = snippet.find('(')?;
    let args_region = &snippet[open + 1..];
    let mut depth: i32 = 1;
    let mut end = 0;
    for (i, ch) in args_region.char_indices() {
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
    let arg = args_region[..end].trim().to_string();

    Some((binding, arg))
}

/// Search the source for the very next `ApiTokenUsage::new(...)` call after
/// `start` and return its argument list as a single string, with outer parens
/// stripped and internal whitespace left intact.
fn find_next_api_token_usage_new(src: &str, start: usize) -> Option<String> {
    let tail = &src[start..];
    let call_offset = tail.find("ApiTokenUsage::new(")?;
    let after_open = call_offset + "ApiTokenUsage::new(".len();
    let args_region = &tail[after_open..];
    let mut depth: i32 = 1;
    let mut end = 0;
    for (i, ch) in args_region.char_indices() {
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
    Some(args_region[..end].to_string())
}

/// Assert that the `n`-th `compute_output_delta(` occurrence in `src` is
/// bound to an identifier that is then passed as the last positional
/// argument to the next `ApiTokenUsage::new(..)` call. The `site_label` is
/// used solely for failure messages.
fn assert_delta_flows_into_api_token_usage(src: &str, needle_ordinal: usize, site_label: &str) {
    let call_idx = nth_occurrence(src, "compute_output_delta(", needle_ordinal)
        .unwrap_or_else(|| panic!(
            "{site_label}: expected at least {needle_ordinal} compute_output_delta(..) call(s) in source"
        ));
    let (binding, _arg) = parse_delta_binding(src, call_idx).unwrap_or_else(|| {
        panic!(
            "{site_label}: compute_output_delta(..) at occurrence {needle_ordinal} is not in the expected \
             `let <BINDING> = session.token_tracker.compute_output_delta(..)` shape — \
             a regression could silently pass 0 to ApiTokenUsage::new"
        )
    });

    let api_call_args = find_next_api_token_usage_new(src, call_idx).unwrap_or_else(|| {
        panic!(
            "{site_label}: no ApiTokenUsage::new(..) found after compute_output_delta(..) call #{needle_ordinal} — \
             the per-turn delta is computed but never passed to the usage record"
        )
    });

    // The binding MUST be the last positional argument. We accept either a
    // bare identifier or a trailing-comma form.
    let trimmed: &str = api_call_args.trim_end_matches(|c: char| c.is_whitespace() || c == ',');
    let last_arg = trimmed.rsplit(',').next().unwrap().trim();
    assert_eq!(
        last_arg, binding,
        "{site_label}: the last positional argument of ApiTokenUsage::new(..) must be the \
         compute_output_delta binding `{binding}`, not `{last_arg}`. A regression to `0` \
         here would silently re-introduce the TOKEN-001 bug."
    );

    // Defense in depth: the argument list must not contain the literal `0` as
    // the output_tokens slot (the last positional arg).
    assert_ne!(
        last_arg, "0",
        "{site_label}: ApiTokenUsage::new(..) is passing literal `0` as output_tokens — \
         this is exactly the TOKEN-001 regression the compute_output_delta helper was \
         supposed to prevent."
    );
}

// ============================================================================
// Scenario: stream_loop.rs main turn finalization site accumulates billing
// ============================================================================

/// Call site: `src/interactive/stream_loop.rs` main turn finalization
/// (around line 1806-1813). This is the FIRST `compute_output_delta(` in the
/// file.
#[test]
fn stream_loop_main_turn_finalization_site_uses_compute_output_delta_result() {
    // @step Given the fspec codelet-cli Rust crate
    // @step When I inspect the main turn finalization call site in stream_loop.rs
    // @step Then the compute_output_delta result is bound to a local variable
    // @step And that variable is passed as the output_tokens arg to ApiTokenUsage::new
    assert_delta_flows_into_api_token_usage(
        STREAM_LOOP_SRC,
        1,
        "stream_loop.rs::run_agent_stream (main turn finalization)",
    );
}

// ============================================================================
// ============================================================================

#[test]
fn gemini_continuation_normal_completion_uses_shared_finalization() {
    // @step Given the fspec codelet-cli Rust crate
    // @step When I inspect the normal-completion call site in gemini_continuation.rs
    // @step Then normal helper completion returns evidence to the shared finalization owner
    let source = gemini_source::gemini_continuation_source();
    assert!(source.contains("return Ok(GeminiContinuationResult::Completed {"));
    let normalized: String = source.split_whitespace().collect();
    assert!(!normalized.contains(".token_tracker.update_from_usage("));
    assert_delta_flows_into_api_token_usage(
        STREAM_LOOP_SRC,
        1,
        "stream_loop.rs shared normal-completion finalization",
    );
}

// ============================================================================
// ============================================================================

#[test]
fn gemini_continuation_exit_restores_settled_billing_without_accumulating_again() {
    // @step Given the fspec codelet-cli Rust crate
    // @step When I inspect the update_token_tracker helper in gemini_continuation.rs
    // @step Then helper exit bookkeeping updates only display values and restores persisted billing
    let source = gemini_source::gemini_continuation_source();
    assert!(source.contains(".update_display_only(&usage, current.output_tokens)"));
    assert!(source.contains("persistence::load_session(session_id)"));
    assert!(source.contains("settled.token_usage.cumulative_billed_output"));
    let normalized: String = source.split_whitespace().collect();
    assert!(!normalized.contains(".token_tracker.update_from_usage("));
}

// ============================================================================
// Sanity: the recovery_compaction site is still wired correctly too.
// This is the call site TOKEN-001 already covers via the
// `flush_partial_state_before_compaction` integration test; this assertion
// keeps the static coverage consistent with the shared finalization site so any
// future reshuffle fails uniformly.
// ============================================================================

#[test]
fn recovery_compaction_flush_site_uses_compute_output_delta_result() {
    assert_delta_flows_into_api_token_usage(
        RECOVERY_COMPACTION_SRC,
        1,
        "recovery_compaction.rs::flush_partial_state_before_compaction",
    );
}
