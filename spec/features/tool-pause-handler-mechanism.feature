@pause-integration
@codelet
@PAUSE-001
Feature: Tool Pause Handler Mechanism

  """
  Core mechanism: codelet/tools/src/tool_pause.rs with PauseKind enum (Continue/Confirm),
  PauseRequest struct, PauseResponse enum, and pause_for_user() blocking function.
  Uses thread-local handlers for per-execution-context isolation.
  """

  Background: User Story
    As a tool developer
    I want a generic pause mechanism that tools can invoke
    So that execution can be paused for user interaction without blocking other sessions

  Scenario: Handler mechanism invokes callback with pause request
    Given a pause handler is registered in the current execution context
    When a tool calls pause_for_user with a Continue request
    Then the handler should be invoked with the request details
    And the handler's response should be returned to the tool

  Scenario: No handler returns Resumed immediately
    Given no pause handler is registered
    When a tool calls pause_for_user
    Then it should return Resumed immediately without blocking

  Scenario: Handler can block and resume
    Given a handler that blocks on a condvar is registered
    When pause_for_user is called
    And a background thread signals Resumed
    Then pause_for_user should unblock and return Resumed

  Scenario: Handler returns Interrupted when user presses Esc
    Given a blocking handler is registered
    When pause_for_user is called
    And the user signals Interrupted
    Then pause_for_user should return Interrupted
