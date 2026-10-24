@provider-abstraction
@PROV-150
@providers
@streaming
@phase-1
Feature: Codex readable reasoning
  Codex users see readable reasoning without duplicated sections or encrypted state.

  """
  Codex normalizes reasoning into the existing frontend thinking delta channel.
  Section identity includes item ID, summary or content index, and text kind.
  Encrypted continuation data remains opaque and is never presentation content.
  Other providers retain their existing rendering and history behavior.
  """

  Scenario: Readable reasoning streams incrementally
    Given raw reasoning and summary text deltas from Codex
    When the frontend consumes the live stream
    Then each readable delta arrives before the answer completes

  Scenario: Completed sections fill missing incremental delivery
    Given summary text done and completed reasoning items without deltas including terminal-only sections
    When the frontend consumes the stream including empty summaries
    Then readable sections appear once and empty sections remain invisible

  Scenario: Repeated completion does not duplicate incremental reasoning
    Given a section delivered through deltas and multiple done events
    When the frontend consumes completed items and the terminal response
    Then only previously unseen readable text appears

  Scenario: Interleaved reasoning preserves section identity
    Given interleaved reasoning items with multiple summary indexes and raw text at the same index
    When the frontend consumes their deltas and completions
    Then section identities and arrival order remain distinct

  Scenario: Encrypted reasoning never reaches presentation or diagnostics
    Given encrypted reasoning alongside readable summaries
    When valid and malformed reasoning events reach the frontend
    Then readable text is visible and encrypted payloads never appear in output or errors

  Scenario: Ordinary Responses thinking remains unchanged
    Given identical done-only reasoning streams for Codex and ordinary Responses
    When both frontends consume the streams
    Then only Codex presents completed reasoning as thinking deltas
