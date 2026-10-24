@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Reconcile ambiguous conversation commits
  """
  Stable envelope UUIDs are physical message UUIDs. Recovery reuses existing
  immutable records and manifest associations under the real writers, rather
  than repeating tools, hooks or appends. Uses the single recovery owner defined
  in recoverable-conversation-persistence.feature.
  """

  Scenario: Message append survives failed manifest promotion
    Given a recovery envelope whose message was written but manifest promotion failed
    When storage is repaired and persistence is retried twice
    Then cold reopen references the same physical message exactly once

  Scenario: Promoted messages survive failed recovery acknowledgement
    Given a recovery envelope whose manifest was promoted but acknowledgement failed
    When storage is repaired and persistence is retried twice
    Then cold reopen references the same physical message exactly once

  Scenario: Recovery admission is bounded and permanent failure remains visible
    Given a real session whose recovery capacity is exhausted
    When another input would exceed recovery capacity
    Then admission fails visibly without evicting previously retained envelopes
    And cold recovery preserves accepted records without dispatching external actions
