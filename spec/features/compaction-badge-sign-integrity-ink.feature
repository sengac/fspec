@done
@context-management
@tui
@header
@ink
@compaction
@CMPCT-040
Feature: Compaction badge sign integrity — TS Ink header never sign-flips a negative reduction
  """
  Clamp at writers, render verbatim: SessionHeader.tsx drops Math.abs(compactionReduction) and renders the prop verbatim; the two raw sessionCompact writers in AgentView.tsx (manual /compact handler and retry dialog) clamp with Math.max(0, result.compressionRatio) so compactionReduction >= 0 when non-null even against a stale unclamped backend. The chunk-path writer (handleCompactionComplete Math.round) is upstream-clamped per CMPCT-039 and untouched. Badge format string unchanged. The Rust ratatui twin is specified in compaction-badge-sign-integrity.feature.
  """

  Background: User Story
    As a TUI user watching the session header after a compaction
    I want to see an honest COMPACTED badge that never sign-flips a negative reduction into a fake positive percentage
    So that I can trust the badge — growth during compaction is never displayed as reduction

  Scenario: TS SessionHeader renders a negative compactionReduction honestly
    Given the Ink SessionHeader receives contextFillPercentage 22.123 and compactionReduction -35
    When the header renders
    Then the output does NOT contain "COMPACTED 35%"
    And the output contains "COMPACTED -35%"

  Scenario: TS raw sessionCompact writers clamp a negative compressionRatio at both write sites
    Given a sessionCompact RPC result whose compressionRatio is negative
    When the handler stores the result
    Then the manual /compact write site stores a compactionReduction clamped to a minimum of 0
    And the retry dialog write site stores a compactionReduction clamped to a minimum of 0
    And no write site stores the raw unclamped compressionRatio
    And the SessionHeader never applies an absolute value to compactionReduction

  Scenario: TS positive compactionReduction still renders unchanged without Math.abs
    Given the Ink SessionHeader receives contextFillPercentage 22.123 and compactionReduction 35.567
    When the header renders
    Then the output contains "[22.12%: COMPACTED 35.57%]"
