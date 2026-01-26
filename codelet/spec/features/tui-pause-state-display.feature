@PAUSE-001
Feature: TUI Pause State Display
  """
  TUI integration for displaying pause state.
  Modifies InputTransition to show pause indicator instead of Thinking.
  Adds isPaused and pauseInfo to useRustSessionState hook.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pause state syncs to React via useRustSessionState hook
  #   2. Paused state replaces Thinking indicator in InputTransition component
  #   3. Two pause kinds: Continue (Enter to resume) and Confirm (Y to approve, N to deny)
  #
  # EXAMPLES:
  #   1. TUI shows 'Thinking...' → tool pauses → TUI shows pause indicator
  #   2. User presses Enter → TUI shows 'Thinking...' again
  #   3. Confirm pause shows warning with Y/N/Esc options
  #
  # ========================================

  Background: TUI State Management
    Given the TUI uses InputTransition component for state display
    And useRustSessionState hook provides isPaused and pauseInfo

  Scenario: TUI transitions from Thinking to Paused and back
    Given the agent is processing a request
    And the TUI shows "Thinking..." with the spinner animation
    When a tool requests a Continue pause
    Then the TUI should replace "Thinking..." with the pause indicator
    And the pause indicator should show the tool name and message
    When the user presses Enter to resume
    Then the TUI should show "Thinking..." again
    When the agent completes processing
    Then the TUI should transition to the input state

  @future
  Scenario: Confirm pause shows approval dialog for dangerous command
    Given the agent is processing a request
    And a tool requests a Confirm pause with message "Potentially dangerous command"
    And the pause details contain "rm -rf /important/*"
    Then the session status should be "paused"
    And the TUI should show a warning with the command text
    And the TUI should show "[Y] Approve [N] Deny [Esc] Cancel"
