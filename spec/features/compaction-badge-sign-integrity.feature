@done
@tui
@agent-view
@header
@store
@ts-parity
@compaction
@CMPCT-040
Feature: Compaction badge sign integrity — Rust header twin never sign-flips a negative reduction
  """
  Clamp at writers, render verbatim: Rust clamps at the single writer dispatch_stream_chunks.rs CompactionComplete handler (.round().max(0.0) as i32); header_build.rs drops r.abs() and states the writer-clamp invariant in its comment. Store setter semantics, the RPC-417 auto-hide timer, and the badge format string are unchanged. The TS Ink twin (SessionHeader.tsx Math.abs removal + AgentView.tsx writer clamps) is specified in compaction-badge-sign-integrity-ink.feature.
  """

  Background: User Story
    As a TUI user watching the session header after a compaction
    I want to see an honest COMPACTED badge that never sign-flips a negative reduction into a fake positive percentage
    So that I can trust the badge — growth during compaction is never displayed as reduction

  Scenario: Rust writer clamps a negative wire reduction to zero
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 1000, compacted_tokens: 2500, compression_ratio: -150.0, turns_summarized: 4, turns_kept: 2 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the stored compaction reduction for "s-1" is 0
    And the SessionHeader text contains "COMPACTED 0%"
    And the SessionHeader text does NOT contain "COMPACTED 150%"
    And the SessionHeader text does NOT contain "COMPACTED -150%"

  Scenario: Rust renderer renders a stored negative reduction verbatim without sign-flipping
    Given session "s-1" is open in AgentView with "s-1" focused
    And a compaction reduction of -35 is forced directly into the store via set_compaction_reduction
    When the App renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "COMPACTED -35%"
    And the SessionHeader text does NOT contain "COMPACTED 35%"

  Scenario: Rust positive wire reduction still renders unchanged with the writer clamp in place
    Given session "s-1" is open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12, turns_kept: 3 } }) is dispatched
    And the App renders the AgentView into a 100x24 TestBackend
    Then the stored compaction reduction for "s-1" is 60
    And the SessionHeader text contains "COMPACTED 60%"
