@pause-integration
@tui
@PAUSE-001
Feature: TUI Pause State Display

  """
  TUI integration: Modify InputTransition.tsx to check isPaused and render
  PauseIndicator instead of ThinkingIndicator when paused. Handle Enter key
  for resume in AgentView.tsx keyboard handler.
  React state sync: Extend useRustSessionState hook to add isPaused boolean
  and pauseInfo to RustSessionSnapshot.
  """

  Background: User Story
    As a user debugging tool execution
    I want to see when a tool is paused and what it's waiting for
    So that I know when to interact with the browser and when to press Enter to continue

  Scenario: TUI transitions from Thinking to Paused and back
    Given the agent is processing a request
    And the TUI shows "Thinking..." with the spinner animation
    When a tool requests a Continue pause
    Then the TUI should replace "Thinking..." with the pause indicator
    And the pause indicator should show the tool name and message
    When the user presses Enter to resume
    Then the TUI should show "Thinking..." again

  Scenario: TUI shows confirm dialog for dangerous operations
    Given the agent is processing a request
    And a tool requests a Confirm pause with message "Potentially dangerous command"
    And the pause details contain "rm -rf /important/*"
    Then the session status should be "paused"
    And the TUI should show a warning with the command text
    And the TUI should show "[Y] Approve [N] Deny [Esc] Cancel"
