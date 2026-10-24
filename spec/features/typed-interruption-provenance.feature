@done
@phase-1
@session
@session-management
@BUG-220
Feature: Typed interruption provenance
  """
  Session control owns the initiating reason. The streaming consumer reads that reason at teardown, records cancelled committed tools, and forwards it through BackgroundOutput. Persisted assistant envelopes retain interruptionReason metadata without reclassifying accepted completion.
  """

  Background: User Story
    As a session owner
    I want to distinguish safety aborts from user stops
    So that cancellation history describes the action that actually occurred

  Scenario: A genuine loop abort survives teardown and persistence
    Given a session whose streaming detector aborts sustained repetitive output
    When the streaming consumer tears down the interrupted response
    Then the interruption event and persisted assistant metadata identify the loop detector

  Scenario: User stop takes precedence in either arrival order
    Given a session receiving a user stop and a genuine detector abort
    When both interruption orders are processed before stream teardown
    Then the final cause is user and no loop recovery note remains

  Scenario: Running tool cancellation preserves its initiating cause
    Given a committed Codex tool batch with a running tool
    When user stop or the loop detector interrupts the running tool
    Then unfinished tools are cancelled exactly once with the actual cause before interrupted and done
