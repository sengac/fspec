@done
@tui
@agent-view
@header
@rpc
@store
@RPC-420
Feature: Compaction reduction display contract — COMPACTED badge and [compaction] notice consume percent-unit compression_ratio directly

  """
  Display fixes: codelet/fspec-tui/src/app/dispatch_stream_chunks.rs:141-142 (badge reduction) and dispatch_slash_commands.rs:287-296 format_compaction_notice — both replace (1.0 - ratio) * 100.0 with the wire value used directly as percent. Backend producers (sessions/handle_impl.rs:335, agent-loop/inject_summary_handler.rs:92, napi twins, background_output.rs:306) already ship percent and are NOT touched.
  Collateral realignment: doc-comment the percent unit on rpc-types CompactionResult.compression_ratio (lib.rs:897); fix stale doc comments in store/agent_view/chrome_state.rs:86-91; keep .abs() at views/agent/header_build.rs:161 with a defensive-parity comment; StubSessionManagerHandle canned 0.5→50.0 (core/src/session_manager_handle.rs:1673) + rpc037_cross_transport_parity.rs assertions; tests/common/mod.rs:749 MockBackend default 1.0→0.0; convert fixtures in slash_compact_rpc047.rs / agentview_session_header_compaction_percentage_rpc100.rs / agentview_compaction_badge_auto_hide_rpc417.rs from fraction-remaining to percent-removed (0.4→60.0, 0.3→70.0, 0.25→75.0, 0.5→50.0) with unchanged asserted display strings. transport/mod.rs:468 default 0.0 stays (renders 0%). Do NOT touch inject_summary_handler measurement basis (CMPCT-038) or the /compact double-notice (RPC-421).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. rpc_types::CompactionResult.compression_ratio is the percent of tokens REMOVED, range [0,100] (producers compute compression_ratio(orig,compacted)*100.0); TUI consumers MUST use the value directly and MUST NOT apply (1.0 - ratio) * 100.0
  #   2. The SessionHeader COMPACTED badge renders round(compression_ratio) as an integer percent: [X%: COMPACTED {round(ratio)}%]; the defensive .abs() in header_build.rs stays (TS Math.abs parity) but must never mask a unit inversion
  #   3. The [compaction] scrollback notice renders compression_ratio directly to one decimal place: "[compaction] {ratio:.1}% reduction (orig → compacted tokens, N turns summarised)" — badge and notice stay coherent from the same wire value
  #   4. Regression guard: a wire compression_ratio of 99.0 MUST render COMPACTED 99% — never 9800% and never a negative value masked by .abs()
  #   5. The 0.0 sentinel (unimplemented transport default at transport/mod.rs:468, empty-session producers) is valid under the percent convention: renders COMPACTED 0% and "0.0% reduction" — no special-casing
  #   6. StubSessionManagerHandle's canned CompactionResult (1000→500) MUST ship compression_ratio 50.0 (percent convention) so the RPC-037 cross-transport parity test exercises the real wire contract on both embedded and WebSocket transports
  #   7. Producer truth: codelet_cli::interactive_helpers::compression_ratio(10000, 4000) * 100.0 == 60.0 feeding the display pipeline MUST render COMPACTED 60% / "60.0% reduction" — display convention is anchored to the real producer formula, not a fixture convention
  #
  # EXAMPLES:
  #   1. StreamChunk::CompactionComplete { compression_ratio: 60.0, original_tokens: 10000, compacted_tokens: 4000, turns_summarized: 12 } on s-1 → header shows COMPACTED 60% AND scrollback notice "[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)"
  #   2. Regression: CompactionComplete with compression_ratio 99.0 → header shows COMPACTED 99% (the pre-fix code computed (1-99)*100 = -9800, masked by .abs() into 9800%)
  #   3. Sentinel: CompactionComplete with compression_ratio 0.0 (e.g. unimplemented-transport default) → header shows COMPACTED 0% and notice shows "0.0% reduction" — never 100%
  #   4. Stub parity: /compact against StubSessionManagerHandle over BOTH embedded and WebSocket transports returns CompactionResult { 1000→500, compression_ratio: 50.0 } identically, and the TUI renders "[compaction] 50.0% reduction (1000 → 500 tokens, 4 turns summarised)"
  #   5. Producer truth: a real compaction of 10000 → 4000 tokens (compression_ratio helper × 100 = 60.0 on the wire) flows through CompactionComplete and the user sees COMPACTED 60% — end-to-end from producer formula to badge
  #
  # ========================================

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to see the COMPACTED header badge and the [compaction] scrollback notice display the wire compression_ratio directly as the percent of tokens removed
    So that the header shows real reductions like COMPACTED 60% instead of impossible values like 9800%, matching every backend producer and the TypeScript reference

  Scenario: CompactionComplete percent value renders directly in the badge and the notice
    Given session "s-1" is open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with fill_percentage 80) has been dispatched
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12, turns_kept: 3 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "[80%: COMPACTED 60%]"
    And the scrollback contains a notice line containing "[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)"

  Scenario: Regression — a 99.0 percent wire value renders COMPACTED 99% and never 9800%
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 100, compression_ratio: 99.0, turns_summarized: 20, turns_kept: 1 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED 99%"
    And the SessionHeader text does NOT contain "9800"

  Scenario: The 0.0 sentinel renders as a zero-percent reduction
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 0, compacted_tokens: 0, compression_ratio: 0.0, turns_summarized: 0, turns_kept: 0 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED 0%"
    And the scrollback contains a notice line containing "0.0% reduction"
    And the scrollback does NOT contain a notice line containing "100.0% reduction"

  Scenario: Stub parity — the canned CompactionResult round-trips as 50.0 percent on both transports
    Given a StubSessionManagerHandle serving both the embedded and WebSocket transports
    When compact_session is called for the same session via each transport
    Then both transports return CompactionResult with original_tokens 1000, compacted_tokens 500, compression_ratio 50.0, turns_summarized 4, turns_kept 2
    And formatting either result yields "[compaction] 50.0% reduction (1000 → 500 tokens, 4 turns summarised)"

  Scenario: Producer formula feeds the display pipeline end-to-end
    Given the wire value is computed by the real producer formula compression_ratio(10000, 4000) * 100.0 equal to 60.0
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete) carrying that wire value is dispatched for session "s-1"
    And the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED 60%"
    And the scrollback contains a notice line containing "60.0% reduction"
