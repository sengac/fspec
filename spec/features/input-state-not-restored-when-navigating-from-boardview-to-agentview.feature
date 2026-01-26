@done
@feature-management
@cli
@input-components
@session-resume
@navigation
@tui
@TUI-051
Feature: Input state not restored when navigating from BoardView to AgentView

  """
  Add useEffect hook in AgentView that watches inputValue and calls sessionSetPendingInput(currentSessionId, inputValue) on every change, debounced to avoid excessive Rust calls. This ensures real-time sync of input state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When navigating from BoardView to AgentView using Shift+Right, the AgentView must restore any pending input stored in Rust via sessionGetPendingInput
  #   2. The input state in Rust should be updated on every keypress via sessionSetPendingInput, not just when switching between screens
  #
  # EXAMPLES:
  #   1. User types 'Hello world' in AgentView, presses Shift+Left to go to BoardView, then presses Shift+Right to return to AgentView, and the input still shows 'Hello world'
  #   2. User types 'Testing 1 2 3' character by character in AgentView, and the input state is synced to Rust after each keystroke via sessionSetPendingInput
  #
  # ========================================

  Background: User Story
    As a user navigating between BoardView and AgentView
    I want to switch from BoardView to AgentView using Shift+Right
    So that my pending input is preserved and restored when I return to the session

  Scenario: Pending input is restored when returning from BoardView to AgentView
    Given the user has typed "Hello world" in the AgentView input area
    And the user presses Shift+Left to navigate to BoardView
    And the user presses Shift+Right to return to AgentView
    Then the input area displays "Hello world"
    And the input content matches what was typed before leaving AgentView

  Scenario: Input state is synced to Rust on every keypress
    Given the user is in AgentView
    When the user types "Testing 1 2 3" character by character
    Then sessionSetPendingInput is called after each keystroke
    And the pending input in Rust matches the current input area content

  Scenario: Pending input is preserved when switching between sessions with Shift+Right/Left
    Given the user has typed "Hello world" in Session A's AgentView input area
    When the user presses Shift+Right to switch to Session B
    And the user presses Shift+Left to return to Session A
    Then Session A's AgentView input area should display "Hello world"

