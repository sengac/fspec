@context-management
@error-handling
@compaction
@resilience
@cli
@rust
@tokens
@TOKEN-001
Feature: Fix cumulative_billed_output never incrementing — all update_from_usage call sites pass output_tokens=0
  """
  TokenTracker::update_from_usage accumulates cumulative_billed_output by the
  output_tokens field of the ApiTokenUsage struct. The four call sites in
  stream_loop.rs, gemini_continuation.rs (two), and recovery_compaction.rs must
  supply a per-turn delta in that field. The cumulative_output argument to
  update_from_usage continues to carry the session-wide cumulative display value.

  Call sites to fix:
  - rust/cli/src/interactive/stream_loop.rs:1808         (main turn finalization)
  - rust/cli/src/interactive/gemini_continuation.rs:324  (Gemini continuation normal completion)
  - rust/cli/src/interactive/gemini_continuation.rs:427  (update_token_tracker helper)
  - rust/cli/src/interactive/recovery_compaction.rs:189  (CMPCT-024 flush on cancel)

  Delta formula:
  let per_turn_delta = current_cumulative_output.saturating_sub(
  session.token_tracker.output_tokens
  );
  ApiTokenUsage::new(input, cache_read, cache_creation, per_turn_delta)
  session.token_tracker.update_from_usage(&usage, current_cumulative_output)

  Note: ApiTokenUsage::new takes 4 positional args (input_tokens, cache_read,
  cache_creation, output_tokens). The session-wide cumulative value is passed
  as the second argument to update_from_usage, NOT to ApiTokenUsage::new.

  SOLID refactor: centralize the delta computation in a single helper (e.g.
  TokenTracker::compute_output_delta(current_cumulative) -> u64) so we don't
  duplicate the saturating_sub at every call site (DRY). All four sites should
  call that helper.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All ApiTokenUsage::new() calls that feed TokenTracker::update_from_usage must pass a real per-turn output_tokens delta instead of literal 0
  #   2. The per-turn delta is computed as saturating_sub(current_cumulative_output, session.token_tracker.output_tokens) so it is always non-negative even when the display ticks backwards
  #   3. The delta computation is centralized in a single TokenTracker helper to satisfy DRY across all four call sites
  #
  # EXAMPLES:
  #   1. A session with two sequential turns (turn1 emits 100 output, turn2 emits 120 output total 220) ends with cumulative_billed_output = 220 rather than 0
  #   2. A stream that reports a lower cumulative (display tick-back) does NOT cause cumulative_billed_output to decrease and does NOT panic on underflow
  #   3. All four call sites use the same centralized helper — grep for saturating_sub(…output_tokens) should find only one call site in production code
  #
  # ========================================
  Background: User Story
    As a session operator
    I want accurate cumulative output-token billing
    So that I can monitor per-session token spend correctly

  Scenario: Cumulative billed output accumulates correctly across two sequential turns
    Given a fresh session where session.token_tracker.output_tokens equals 0 and cumulative_billed_output equals 0
    When turn one completes and reports a cumulative output of 100 tokens
    And turn two completes and reports a cumulative output of 220 tokens
    Then session.token_tracker.cumulative_billed_output should equal 220
    And session.token_tracker.output_tokens should equal 220

  Scenario: Per-turn delta never underflows when the cumulative display ticks backward
    Given a session where session.token_tracker.output_tokens equals 500 and cumulative_billed_output equals 500
    When the stream reports a cumulative output of 300 tokens
    Then session.token_tracker.cumulative_billed_output should remain 500
    And session.token_tracker.output_tokens should equal the reported cumulative
    And no panic occurs from integer underflow

  Scenario: Delta computation is centralized in a single helper
    Given the fspec codelet-cli Rust crate
    When I search the interactive module for `saturating_sub` applied to `.output_tokens` fields
    Then the only production-code match is inside the single TokenTracker helper method
    And all four update_from_usage call sites delegate to that helper
