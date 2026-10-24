@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Background request counters survive delayed committed context events
  """
  The shared committed-compaction-context-projection contract applies. Delayed
  or stale events cannot replace current measured request context, and duplicate
  completion snapshots must not generate duplicate completion delivery.
  """

  Scenario: Delayed completion preserves newer background request counters
    Given background counters describe a measured request from a committed generation
    When its delayed committed snapshot and an older request snapshot arrive
    Then the background counters retain the measured request values
    And observers receive the completion once without receiving the stale request
