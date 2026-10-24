@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Export causally ordered transcript records in pinned archival pages
  """
  The canonical archive merges private display records with authoritative
  originals, preserving journal identity and durable order rather than clocks.
  Snapshot continuation pins both source lanes and never reinterprets a prefix
  after later original promotion.
  """

  Scenario: Canonical archival export preserves causal journal identities
    Given two original requests with journal outcomes and backward timestamps
    When the canonical archival export is reassembled
    Then each outcome follows its original content before the next request
    And journal identities timestamps and inert partial content survive without model contamination
    And aborted response attempts remain absent from archival dialogue
    And inconsistent causal anchors fail explicitly instead of guessing event order

  Scenario: Canonical archival snapshots pin the journal prefix
    Given an unpromoted journal prefix spanning bounded UTF-8 archive pages
    When a later journal outcome and original promotion are committed during traversal
    Then the pinned export retains the original prefix once without later events
    And a fresh export includes the outcome and suppresses promoted duplicate content
    And changing an acknowledged journal prefix invalidates the pinned export
