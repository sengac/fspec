@BUG-229
@phase-1
@agent-core
@context-management
@compaction
Feature: Codex compactor occupancy runway
  """
  Physical Codex continuations retain independent complete-request admission.
  The compactor recovers only typed capacity exhaustion, using its full gathered
  history including the final tool evidence. Uncommitted generated continuation
  output is not authoritative source evidence and cannot publish a candidate.
  The existing DAG validation and durable publication gates remain mandatory.
  """

  Scenario: Codex continuation capacity recovery retains final tool evidence
    Given a streaming compactor has gathered a persisted original evidence page
    When replayed Codex output makes a physical exploration continuation exceed capacity
    Then the rejected continuation never reaches transport and tool-free synthesis retains the complete page
    And the selected model and output reservation remain unchanged

  Scenario: Codex physical requests respect the exact incident boundary
    Given a Codex request has capacity 1050000 and output reservation 128000
    When complete initial and continuation requests contain 922000 or 922001 estimated input units
    Then only requests at or below 922000 reach transport
