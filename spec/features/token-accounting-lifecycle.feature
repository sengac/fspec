@validation
@phase-1
@session-management
@done
@agent-core
@cli
@context-management
@tokens
@compaction
@bug-fix
@BUG-236
Feature: Token accounting lifecycle preserves evidence and estimates
  """
  Registration captures a compaction basis before inference, so zero input alone
  cannot establish a provider omission. Track input provenance independently of
  numeric display snapshots: uninitialized, unknown legacy, reported, unavailable
  in an observed response, restored, reset, and content estimated. Streaming and
  final-response accounting must record actual observations; seeded displays and
  pre-compaction flushes must not invent observations. Preserve cache-inclusive
  totals and the existing content estimator and reduction-ratio contract.
  Incident logs establish startup timing but do not establish why longer-history
  trackers were zero. Durable restore has no persisted provenance, so restored
  values remain explicitly restored rather than asserted provider measurements.
  """

  Background: User Story
    As a session user
    I want token estimates distinguished from observed accounting
    So that compaction measurements and diagnostics remain truthful

  Scenario: Fresh registration estimates content without an accounting warning
    Given a fresh session with two messages before inference
    When compaction handlers repeatedly capture a token basis
    Then the basis equals the content estimate without warnings or provider omission claims
    And accounting remains uninitialized

  Scenario: Reported input preserves cache inclusive totals exactly once
    Given a response reports 30000 fresh input and 150000 cache read tokens
    When the tracker records the response and a compaction basis is captured
    Then the basis is 180000 with reported provenance and unchanged billing semantics

  Scenario: Observed unavailable input uses an honest diagnostic and estimate
    Given a prior response reported input tokens
    When a completed response has no usable input accounting
    Then the old input is not reused as a current measurement
    And the content estimate is accompanied by an unavailable accounting warning without an asserted provider cause

  Scenario: Display snapshots and partial flushes do not invent observations
    Given a fresh or content estimated tracker and a seeded streaming display
    When display state is saved and partial state is flushed before a response
    Then input provenance is unchanged and no missing usage observation is invented
    And cache inclusive totals and existing flush billing are preserved

  Scenario: Restored accounting is not relabeled as provider reported
    Given durable sessions with zero or nonzero saved input accounting
    When their accounting is restored and a compaction basis is captured
    Then saved nonzero totals are preserved and zero totals use content estimates
    And diagnostics identify restored accounting without provider omission warnings

  Scenario: Legacy accounting without provenance remains unknown
    Given serialized trackers without lifecycle metadata
    When trackers with zero or nonzero input are loaded
    Then provenance is unknown and numeric totals or content estimates remain usable without provider omission warnings

  Scenario: Reset accounting does not claim a missing provider response
    Given a tracker with previously reported input and billing
    When accounting is reset after compaction
    Then provenance is reset and no old provider observation is asserted
    And session spend is preserved

  Scenario: Post compaction recalculation is explicitly estimated
    Given a session with reported input and cache tokens
    When compacted message content replaces the history and accounting is recalculated
    Then the basis equals the new content estimate with estimated provenance
    And stale cache fields are cleared without changing session spend

  Scenario: Empty content legitimately has a zero basis
    Given uninitialized or unavailable accounting with empty message content
    When a compaction basis is captured
    Then the basis is zero without warnings or invented tokens

  Scenario: Selection reset invalidates old input evidence
    Given a session with reported input for its previous selection
    When provider state is reset for a model or provider selection change
    Then old input evidence is invalidated without claiming a provider omission
    And retained content remains measurable by estimation
