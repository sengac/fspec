@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Committed context wire snapshots retain unknown coverage and occupancy
  """
  The shared committed-compaction-context-projection contract applies. RPC
  serialization preserves explicitly unknown coverage, generation and tokens;
  reduction compares only context occupancy, never cumulative billed spend.
  """

  Scenario: Unknown durable coverage survives the committed snapshot wire roundtrip
    Given a committed context wire snapshot with unknown durable coverage
    When the snapshot is decoded and encoded again
    Then coverage remains unknown and generation and tokens remain unchanged

  Scenario: Reduction uses context occupancy independently of displayed spend
    Given a committed context snapshot with separate occupancy and cumulative spend
    When its reduction is calculated
    Then only its context occupancy is compared with the original context
