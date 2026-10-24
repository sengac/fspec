@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Pending publication restore cannot fabricate completed reconciliation
  """
  The shared committed-compaction-context-projection contract applies. Pending
  publication preserves the authoritative durable generation and all saved
  counters while blocking inference and suppressing successful refresh notices.
  """

  Scenario: Pending publication restore cannot claim completed reconciliation
    Given a durable generation still awaiting completion publication
    When a fresh session restores its saved context and spend
    Then no reconciled committed snapshot or completion notice is emitted
    And the pending generation remains authoritative without losing saved counters
