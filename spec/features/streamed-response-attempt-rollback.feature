@phase-1
@cli
@providers
@provider-abstraction
@regression
Feature: Abandoned response attempts do not become conversation history
  """
  Architecture notes:
  Physical attempt boundaries pass through provider streams, the agent engine,
  and output consumers. Consumers keep live streaming but roll back abandoned
  attempt text and reasoning before receiving replacement output.
  Successfully completed earlier segments and tool results remain authoritative.
  """

  Scenario: A reconnected response can be followed by another turn
    Given a Codex response streams text before its connection is lost
    And its replacement response completes successfully
    When the streaming agent finishes the turn
    Then only the successful attempt remains in conversation history
    And the next request passes checkpoint ancestry validation
