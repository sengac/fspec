@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Cold reconciliation preserves saved spend and request context separately
  """
  The shared committed-compaction-context-projection contract applies. A fresh
  process restores current occupancy and every saved billing, cache and reasoning
  counter without confusing cumulative spend with context reduction or fit.
  """

  Scenario: Cold reconciliation preserves saved spend and newer context occupancy
    Given a committed DAG followed by saved request occupancy billing cache and reasoning
    When a fresh session restores its authoritative token state
    Then its live cached and emitted counters retain every saved value
    And its reconciled context fill uses saved occupancy rather than the original commit estimate
    And its reduction excludes cumulative billing and reasoning spend
