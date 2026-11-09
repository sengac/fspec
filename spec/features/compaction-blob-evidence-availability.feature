@done
@BUG-528
@persistence
@context-management
@compaction
@rust
Feature: Compaction blob evidence availability
  """
  Compaction never builds on blob-backed evidence it cannot restore exactly. A
  blob-backed original carries _blobRefs (key and SHA-256 hash) and blob
  markers in its message content; restore_original_blobs
  (rust/core/src/persistence/original_blobs.rs) resolves each marker from blob
  storage and rejects a missing or corrupt blob, a reference whose hash or key
  does not match, malformed or duplicate references, and blob bytes that are
  not valid for their slot (non-UTF-8 text, non-JSON tool input). Source
  capture, promotion of a candidate, reconciliation of a commit, publication
  and cold replay all go through it, and each rejects damaged evidence with
  CompactionCommitError::Rejected without changing a byte on disk. Valid
  evidence of every kind (tool result, tool input, reasoning, image, document)
  is restored exactly by cold replay, again without changing a byte.
  The tests came with 2699693b7 and named no feature; BUG-528 wrote this one.
  Each scenario states what its test checks; the sixteen damage cases are the
  ten blob deletions or corruptions (two per evidence kind) and the six
  reference faults listed above. Since BUG-494 each snapshot is taken with
  the message index and history sharing one timestamp tick (BUG-519).
  Tests: rust/core/tests/compaction_blob_availability.rs.
  """

  Background: User Story
    As a session owner whose conversation is compacted
    I want to have compaction refuse blob-backed evidence it cannot restore exactly
    So that a compacted conversation never silently loses tool results, inputs, reasoning or attachments

  Scenario: Source capture rejects unavailable blob evidence
    Given a session whose blob-backed evidence is damaged in each of the sixteen ways in turn
    When compaction source capture reads the session after a store reset
    Then capture is rejected for every damage case
    And no file in the data directory changes

  Scenario: Promotion rejects unavailable blob evidence
    Given a captured compaction candidate whose blob-backed evidence is then damaged in each of the sixteen ways in turn
    When the candidate is promoted after a store reset
    Then promotion is rejected for every damage case
    And no file in the data directory changes

  Scenario: Reconciliation rejects unavailable blob evidence
    Given a committed compaction whose blob-backed evidence is then damaged in each of the sixteen ways in turn
    When the commit is reconciled after a store reset
    Then reconciliation is rejected for every damage case
    And no file in the data directory changes

  Scenario: Publication rejects unavailable blob evidence
    Given a committed compaction whose blob-backed evidence is then damaged in each of the sixteen ways in turn
    When the committed compaction is published after a store reset
    Then publication is rejected before the publisher runs for every damage case
    And no file in the data directory changes

  Scenario: Cold replay rejects unavailable covered blob evidence
    Given a committed and reconciled compaction whose covered blob-backed evidence is then damaged in each of the sixteen ways in turn
    When the session's envelopes are replayed after a store reset
    Then replay is rejected for every damage case
    And no file in the data directory changes

  Scenario: Valid blob evidence preserves original bytes and identities
    Given a session with blob-backed evidence of every kind and a literal blob-like text message
    When the compaction is captured, committed, published and replayed cold
    Then capture leaves the data directory unchanged
    And replay restores each original payload and blob reference
    And replay leaves the messages, the blobs and the data directory unchanged
