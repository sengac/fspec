@done
@agent-core
@context-management
@rpc
@compaction
@CMPCT-039
Feature: Negative compression_ratio reachable on the wire — clamp missing in shared helper, three live unclamped producers
  """
  Fix lives in the shared helper compression_ratio() (codelet/cli/src/interactive_helpers.rs:190-196) via .clamp(0.0, 1.0). Post-RPC-421 reality: the live callers are BOTH inject_summary_handler twins (agent-loop/src/inject_summary_handler.rs and napi/src/inject_summary_handler.rs — the StreamChunk::CompactionComplete producers whose honest post-injection numbers feed every notice/badge path that still formats a ratio) plus the debug-only recovery_compaction.rs:450 capture. The compact_session RPC twins (sessions/src/handle_impl.rs, napi/src/session_bindings.rs session_compact) and the plain-CLI repl_loop.rs /compact handler NO LONGER call the helper — RPC-421 made them ship the acknowledgement sentinel (compression_ratio 0.0, compacted_tokens 0) directly, and the REPL prints no ratio at all. The clamp still protects every surface that renders a ratio because all such surfaces source from the CompactionComplete chunk. TUI-side clamping would be the wrong layer per the RPC-420 wire contract.
  """

  Background: User Story
    As a TUI or CLI user watching compaction results
    I want to always see a non-negative compression ratio on every wire and display surface
    So that context growth in tiny sessions is reported as 0% compression instead of a nonsensical negative percentage

  Scenario: Helper clamps a context-growth ratio to zero
    Given a compaction where the compacted token count 1600 exceeds the original token count 1000
    When the compression ratio is calculated by the shared helper
    Then the helper returns exactly 0.0
    And the helper never returns a negative value

  Scenario: Helper reports a normal reduction unchanged
    Given a compaction where the original token count 1000 shrinks to a compacted token count of 400
    When the compression ratio is calculated by the shared helper
    Then the helper returns 0.6 representing 60 percent of tokens removed

  Scenario: Helper returns zero when the original token count is zero
    Given a compaction where the original token count is 0
    When the compression ratio is calculated by the shared helper
    Then the helper returns exactly 0.0 via the division guard

  Scenario: Helper returns zero when compacted tokens equal original tokens
    Given a compaction where the compacted token count equals the original token count of 800
    When the compression ratio is calculated by the shared helper
    Then the helper returns exactly 0.0

  Scenario: compact_session RPC result never ships a negative ratio when context grows
    Given a tiny session whose injected compaction instruction exceeds its original token count
    When the user compacts the session through the session manager handle
    Then the returned CompactionResult compression_ratio is exactly 0.0
    And the returned CompactionResult compression_ratio is not negative

  Scenario: NAPI session_compact result never ships a negative ratio when context grows
    Given a tiny NAPI session whose injected compaction instruction exceeds its original token count
    When the user compacts the session through the NAPI session_compact binding
    Then the returned CompactionResult compression_ratio is exactly 0.0
    And the returned CompactionResult compression_ratio is not negative
