@CMPCT-069
@phase-1
@agent-core
@context-management
@compaction
Feature: Complete token event forwarding
  Token event projections retain measured usage detail independently of settlement.

  """
  Architecture: Forward the supported typed token fields without deriving usage
  from display counters. Absent measurements remain null and measured zero remains
  zero. Physical-request settlement is separately owned by the durable request ledger.
  """

  Scenario: Measured token fields survive JSON forwarding
    Given a typed token event with cache reasoning and cumulative billing measurements
    When the bridge projects the event to JSON
    Then every supported measurement survives unchanged

  Scenario: Unknown token fields remain unknown in JSON
    Given a typed token event with absent cache reasoning and cumulative billing measurements
    When the bridge projects the event to JSON
    Then absent measurements remain null rather than fabricated zero
