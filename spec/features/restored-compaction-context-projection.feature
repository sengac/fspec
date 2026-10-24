@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Restored committed context refreshes without claiming notice delivery
  """
  The shared committed-compaction-context-projection contract applies. Restore
  projects the authoritative durable generation without fabricating completion
  delivery, duplicating notices or replacing newer measured request state.
  """

  Scenario: Restored committed context refreshes without claiming completion delivery
    Given a saved committed generation with estimated occupancy and model limits
    When the session restores its authoritative token state
    Then the background context snapshot immediately identifies that durable generation
    And the refresh is reconciliation rather than a delivered completion notice
    And repeated restoration does not duplicate the refresh
