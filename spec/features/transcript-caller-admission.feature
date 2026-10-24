@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Retain the caller delta when bounded shared admission is full
  """
  The caller retains its unacknowledged original record until the shared bounded
  owner can admit it. Storage failure is not a genuine user cancellation.
  Recovery retains authentic provider output as an original independently of
  whether publication was admitted, as required by BUG-246 ownership recovery.
  """

  Scenario: Full recovery admission retains the caller's original delta
    Given a full bounded recovery owner and an unacknowledged assistant delta
    When storage is repaired and the same output completes recovery
    Then the original delta is archived once without being published or lost
    And persistence interruption is not labelled user cancellation
    And the recovered conversation contains the authentic prefix exactly once
