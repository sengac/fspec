@done
@CMPCT-068
@phase-1
@agent-core
@compaction
@context-management
@bug
@high
Feature: Background reconciliation republishes authoritative committed context
  """
  Lost completion notification leaves the durable generation authoritative and
  inference blocked. Exact reconciliation must refresh the background projection
  from saved occupancy and spend without promoting again or inventing completion
  delivery. Repeated reconciliation must not duplicate the context refresh.
  """

  Scenario: Cold background reconciliation refreshes context without duplicate completion
    Given a cold background session with pending publication and saved occupancy and spend
    When an incorrect identity is reconciled
    Then inference remains blocked and no committed context is published
    When the authoritative identity is reconciled twice
    Then one reconciled snapshot preserves saved counters coverage and generation
    And no completion notice or second durable promotion is produced
