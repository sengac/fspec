@done
@header
@session
@session-resume
@context-management
@tokens
@TOKEN-003
Feature: SessionHeader reasoning (🧠) tokens do not accumulate — each API usage overwrites the last value
  """
  Fix is backend-only (TUI last-value mirror is correct per RPC-099 and must not change). StreamingTokenDisplay (rust/core/src/streaming_display/streaming_token_display.rs) gains a reasoning cumulative structure mirroring OutputTokenTracker: reasoning_cumulative_base + current-segment reasoning; start_new_segment() accumulates the previous segment's reasoning into the base. Constructors gain a prev_reasoning seed (threaded from session.token_tracker.reasoning_tokens at the 6 from_cache_inclusive_total sites in stream_loop.rs and 2 sites in gemini_continuation.rs). End-of-turn updates (stream_loop.rs ~2252, gemini_continuation.rs update_token_tracker) carry final_display.reasoning_tokens into ApiTokenUsage via with_reasoning_tokens so session.token_tracker.reasoning_tokens holds the session cumulative. TokenTracker::reset_after_compaction() keeps the cumulative reasoning (session-spend metric). Persistence: add reasoning_tokens to manifest TokenUsage (rust/core/src/persistence/manifest.rs:44) and extend persist_token_state/update_session_tokens/set_session_tokens in both rust/agent-loop/src/persist.rs and rust/napi/src/persist.rs; restore on /resume. Compaction threshold math is unchanged (uses current-segment physical context).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The displayed reasoning token count is SESSION-CUMULATIVE: it accumulates across API segments within a turn (tool loops) and across turns, matching the output (↑) counter semantics and the Codex reference implementation
  #   2. StreamingTokenDisplay must track reasoning tokens with the same cumulative structure as output tokens: a cumulative base plus a current-segment value, where start_new_segment() accumulates the previous segment's reasoning into the base before resetting the current-segment value
  #   3. The end-of-turn session tracker update (stream_loop.rs ~2252 and gemini_continuation.rs update_token_tracker) must carry the turn's cumulative reasoning value into ApiTokenUsage (via with_reasoning_tokens or an equivalent builder) so session.token_tracker.reasoning_tokens holds the session cumulative instead of being zeroed to 0
  #   4. Every StreamingTokenDisplay seed site (the 6 from_cache_inclusive_total sites in stream_loop.rs and the 2 sites in gemini_continuation.rs) must seed the previous session reasoning value from session.token_tracker.reasoning_tokens so the new turn continues from the correct cumulative value
  #   5. The reasoning token value must be persisted to the session manifest (TokenUsage struct) and restored on /resume, matching how input/output tokens are persisted via persist_token_state
  #   6. The 🧠 counter must be monotonically non-decreasing within a session (never ticks backward), and must not leak into compaction threshold math — threshold checks use physical context occupancy (input + current-segment output + current-segment reasoning), not the cumulative reasoning display value
  #
  # EXAMPLES:
  #   1. Turn 1 uses 800 reasoning tokens. Turn 2 uses 200 reasoning tokens. After turn 2 the header shows 1000🧠 — the previous turn's value is kept and accumulated, not replaced
  #   2. A session accumulates 1000 reasoning tokens across several turns, is closed, and is restored via /resume — the header shows 1000🧠 again (the value was persisted to the session manifest)
  #   3. A provider that reports no reasoning tokens (e.g. a non-thinking model) shows no 🧠 suffix at all — the counter stays 0 and the suffix is omitted, exactly as today
  #
  # ASSUMPTIONS:
  #   1. reset_after_compaction() keeps the cumulative reasoning value (like cumulative_billed_*), because it is a session-spend metric rather than a context metric; the TUI context-fill recompute uses the wire cumulative reasoning but the backend emit_context_fill_from_usage remains authoritative for the [X%] badge
  #   2. Estimating reasoning tokens from ReasoningDelta text for Anthropic/Gemini (where the API folds thinking tokens into output_tokens) is OUT OF SCOPE for this card — it becomes a separate follow-up work unit
  #
  # ========================================
  Background: User Story
    As a developer using extended-thinking models
    I want to see a session-cumulative reasoning (🧠) token counter in the SessionHeader that accumulates across API segments and turns
    So that I can monitor total reasoning spend per session with the same semantics as the output (↑) counter, and the value survives /resume

  Scenario: Reasoning tokens accumulate across API segments within a single turn
    Given a session using a provider that reports reasoning tokens
    And a turn with two API segments where segment 1 reports 500 reasoning tokens and segment 2 reports 300 reasoning tokens
    When the turn completes
    Then the session reasoning total is 800
    And the SessionHeader displays "800🧠" (not "300🧠")

  Scenario: Reasoning tokens accumulate across turns
    Given a session where turn 1 used 800 reasoning tokens
    When turn 2 completes and uses 200 reasoning tokens
    Then the SessionHeader displays "1000🧠"
    And the previous turn's value is kept and accumulated, not replaced

  Scenario: Reasoning tokens persist across session restore
    Given a session that accumulated 1000 reasoning tokens across several turns
    When the session is closed and restored via /resume
    Then the SessionHeader displays "1000🧠" again
    And the reasoning value was persisted to the session manifest

  Scenario: No reasoning suffix when the provider reports no reasoning tokens
    Given a session using a provider that reports no reasoning tokens
    When turns complete with zero reasoning tokens
    Then the SessionHeader displays no 🧠 suffix
    And the counter stays 0, exactly as today

  Scenario: Reasoning counter never ticks backward within a session
    Given a session with 1000 cumulative reasoning tokens
    When a new turn reports a lower per-segment reasoning value
    Then the displayed reasoning total is still at least 1000
    And the counter never decreases

  Scenario: Cumulative reasoning does not affect compaction threshold math
    Given a session with a large cumulative reasoning total but small current context occupancy
    When the context fill percentage is computed for the [X%] badge
    Then the threshold check uses physical context occupancy (input + current-segment output + current-segment reasoning)
    And the cumulative reasoning display value does not inflate the fill percentage
