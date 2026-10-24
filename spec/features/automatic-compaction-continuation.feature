@BUG-210
@phase-1
@agent-core
@context-management
@compaction
Feature: Automatic compaction preserves unfinished agent work
  """
  Terminal overflow recovery owns the captured history until its durable DAG is
  committed, published, and reconciled. Resume ordinary inference on that reduced
  history, not the rejected request. Keep a finite recovery budget across synthetic
  continuations; cancellation and failed publication must not reopen inference.
  NAPI parity and session-state transitions are specified in their own features.
  """

  Background: User Story
    As a user running an agent session
    I want unfinished work to resume after automatic compaction
    So that I do not need to manually ask the agent to continue

  Scenario: Overflow recovery resumes work without another user message
    Given an agent with completed work encounters a context overflow
    When automatic compaction commits a usable summary
    Then the agent continues the original task on the reduced history
    And completed work is not repeated and completion is not announced prematurely

  Scenario: Repeated overflow stops at a finite recovery budget
    Given an agent repeatedly exceeds its context after successful compaction
    When the automatic recovery budget is exhausted
    Then the user receives an explicit terminal recovery error
    And no further compaction or inference request starts

  Scenario: Cancelling compaction does not restart the agent
    Given automatic compaction is waiting for its summary
    When the user cancels the session
    Then the session stops without another inference request
    And its previously committed conversation remains available

  Scenario: Failed compaction does not restart the agent
    Given automatic compaction cannot produce a usable summary
    When the failed recovery finishes
    Then the user receives an actionable error without another inference request
    And its previously committed conversation remains available

  Scenario: A new user task receives a fresh recovery budget
    Given an earlier task exhausted its automatic recovery budget
    When the user submits a new task
    Then the new task can recover from a context overflow

  Scenario: Cancellation after summary generation prevents automatic continuation
    Given an automatic summary completes as the user cancels the session
    When the outer loop considers continuing the interrupted task
    Then no continuation inference request starts

  Scenario: Loop-detector interruption still permits its corrective retry
    Given an agent is interrupted by the repetitive-output detector rather than the user
    When the outer loop schedules its corrective retry
    Then inference resumes without another user message
    And the next request includes the corrective note and finishes the task

  Scenario: An active agent resumes after submitting its own summary
    Given an active agent requests compaction of its own unfinished task
    When its submitted summary is durably applied by the outer loop
    Then the agent resumes on that summary without another user message
    And completion is announced only after the remaining task finishes
