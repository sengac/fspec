@done
@BUG-243
@phase-1
@persistence
@context-management
Feature: Page full session archives independently from model replay
  """
  Versioned cursors pin a session manifest prefix and exclusive snapshot end.
  UTF-8-safe canonical JSON fragments reassemble losslessly within a bounded
  serialized page budget. Native checkpoints have identity-only markers.
  Archive reads never install model history or alter compaction state.
  """

  Background: User Story
    As a user reopening a compacted conversation
    I want all original dialogue through bounded archival pages
    So that display history remains complete without expanding inference context

  Scenario: Page full archival dialogue independently from reduced model replay
    Given a compacted session with large UTF-8 dialogue and legacy provider state
    When bounded archival fragments traverse a pinned original snapshot
    Then complete original dialogue and persisted identities survive reassembly
    And provider state is explicitly non-conversational and native payloads stay excluded
    And model replay still contains only the committed summary and uncovered suffix
    And unavailable or changed original sources fail explicitly
