@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Runtime candidate rejection cannot publish committed context
  """
  The shared committed-compaction-context-projection contract applies. Retain
  immutable originals and the last committed generation on typed failure, block
  unsafe inference, and never substitute a fallback or emit false completion.
  """

  Scenario: Runtime candidate application cannot announce an uncommitted generation
    Given a submitted DAG candidate without durable promotion
    When the runtime attempts deferred candidate application
    Then no committed snapshot or completion notice is published
    And the authoritative context counters remain unchanged
