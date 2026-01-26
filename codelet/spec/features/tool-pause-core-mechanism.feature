@PAUSE-001
Feature: Tool Pause Core Mechanism
  """
  Core pause/resume mechanism in Rust. PauseKind enum (Continue/Confirm),
  PauseState struct, pause_for_user() blocking function with Condvar synchronization.
  
  This is the foundation that WebSearchTool and future tools build upon.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Two pause kinds: Continue (Enter to resume) and Confirm (Y to approve, N to deny)
  #   2. Esc key interrupts the agent during any pause (same as during normal execution)
  #   3. Rust is the single source of truth for pause state - SessionStatus::Paused with PauseState struct
  #   4. Single pause per tool call - no multiple pause points within one execution
  #
  # EXAMPLES:
  #   1. Tool calls pause_for_user() → blocks until resume/approve/deny/interrupt
  #   2. Resume clears pause state and returns PauseResponse::Resumed
  #   3. Interrupt returns PauseResponse::Interrupted
  #   4. Confirm pause: Y returns Approved, N returns Denied
  #
  # ========================================

  Background: Tool Pause API
    Given the tool pause module provides pause_for_user function
    And pause kinds are Continue and Confirm
    And pause responses are Resumed, Approved, Denied, and Interrupted

  Scenario: Pause state is managed in Rust and synced to React
    Given a session exists with status "running"
    When a tool calls pause_for_user with kind "continue" and message "Page loaded"
    Then the session status should change to "paused"
    And the pause state should contain kind "continue"
    And the pause state should contain the tool name and message
    When React calls sessionGetStatus
    Then it should return "paused"
    When React calls sessionGetPauseState
    Then it should return the pause info with kind, tool name, and message

  Scenario: Resume clears pause state and returns to running
    Given a session is paused with a Continue pause
    When the user triggers resume via sessionPauseResume
    Then the pause state should be cleared
    And the session status should change to "running"
    And the blocked tool should receive PauseResponse::Resumed

  Scenario: User interrupts during pause with Escape key
    Given a tool is paused with a Continue pause
    When the user presses Escape
    Then the tool should receive PauseResponse::Interrupted
    And the session status should be "interrupted"

  @future
  Scenario: User approves Confirm pause with Y key
    Given a tool is paused with a Confirm pause
    When the user presses Y
    Then the tool should receive PauseResponse::Approved
    And the tool should proceed with the operation

  @future
  Scenario: User denies Confirm pause with N key
    Given a tool is paused with a Confirm pause
    When the user presses N
    Then the tool should receive PauseResponse::Denied
    And the tool should return a denial error without executing the operation
