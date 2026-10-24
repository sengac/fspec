@BUG-213
@phase-1
@cli
@compaction
@context-management
Feature: Compaction entry routes report unknown work honestly
  """
  Architecture notes:
  The truthful-compaction-activity lifecycle contract applies. Existing
  pre-prompt and context-limit boundaries and the manual working-view builder
  publish reached phases with unknown current=0,total=0 work counts. Active
  messages and approximate turns are not measured completed-work units.
  Exercise real route preparation and the pending builder without networking.
  This specification maps only to truthful_compaction_routes_bug213.rs.
  """

  Scenario Outline: Every compaction entry route reports unknown work honestly
    Given a session requiring <route> compaction without measured completed-work units
    When that route begins its real compaction activity
    Then its activity reports the phase actually reached and unknown work counts
    And no active-message count, approximate-turn count, or synthetic total becomes work progress

    Examples:
      | route         |
      | pre-prompt    |
      | context-limit |
      | manual        |
