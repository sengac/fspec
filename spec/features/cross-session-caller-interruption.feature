@agent-core
@phase-1
@context-management
@compaction
@CMPCT-056
Feature: Caller interruption releases cross-session generation ownership
  """
  The cross-session-compaction-ownership contract applies. A requesting caller's
  cancellation releases only its operation ownership, retains committed target
  context and restores actual prior activity without successful completion.
  """

  Scenario: Caller interruption cancels cross-target generation
    Given an Idle target with a cross-target builder waiting at a barrier
    When the requesting caller is interrupted without aborting its task
    Then the operation returns cancellation within the harness deadline
    And the unchanged target returns to Idle without completion
