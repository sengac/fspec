@testing
@cli-integration
@session-management
@tui
@TUI-053
Feature: Input not restored when switching between sessions
  """
  The resumeSessionById function restores pending input by calling sessionGetPendingInput and setInputValue with the result (or empty string)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When navigating between sessions with Shift+Right/Left, the resumeSessionById function must restore the correct pending input for each session
  #   2. The resumeSessionById function must always call setInputValue with the pending input (or empty string) even if no pending input exists for a session
  #
  # EXAMPLES:
  #   1. User types "Hello world" in Session A, presses Shift+Right to switch to Session B, then presses Shift+Left to return to Session A, and Session A's input still shows "Hello world"
  #
  # ========================================
  Background: User Story
    As a user switching between AgentView sessions
    I want to navigate between sessions with Shift+Right/Left
    So that pending input is preserved and restored for each session

  Scenario: Pending input is preserved when switching between sessions with Shift+Right/Left
    Given the user has typed "Hello world" in Session A's AgentView input area
    When the user presses Shift+Right to switch to Session B
    And the user presses Shift+Left to return to Session A
    Then Session A's AgentView input area should display "Hello world"

  Scenario: Empty input is correctly set when switching to session with no pending input
    Given the user has typed "Hello world" in Session A's AgentView input area
    When the user presses Shift+Right to switch to Session B
    Then Session B's AgentView input area should be empty

  Scenario: Different pending inputs are preserved for each session
    Given the user has typed "Hello world" in Session A's AgentView input area
    And the user has typed "Goodbye" in Session B's AgentView input area
    When the user presses Shift+Right to switch to Session B
    Then Session B's AgentView input area should display "Goodbye"
    And the user presses Shift+Left to return to Session A
    And Session A's AgentView input area should display "Hello world"
