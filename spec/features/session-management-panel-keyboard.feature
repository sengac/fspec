@GIT-035
Feature: Session Management Panel keyboard input
  """
  Refactor SessionManagementPanel from Dialog overlay to Full-Screen View pattern.

  Reference implementations:
  - AgentView isResumeMode (line 7410-7500)
  - WatcherCreateView.tsx

  Full-Screen View Pattern:
  - Component receives terminalWidth/terminalHeight props
  - Uses position=absolute with 100% dimensions
  - Single useInputCompat with CRITICAL priority
  - ESC to close, returns to previous view
  - Inline confirmation prompts instead of dialog overlays
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Session Management Panel MUST be a full-screen view (not a dialog overlay)
  #   2. Session Management Panel MUST respond to keyboard input: ↑↓ for navigation, M for merge, D for discard, R for refresh, Esc to close
  #   3. Confirmations MUST use inline prompts at bottom of screen, not dialog overlays
  #
  # EXAMPLES:
  #   1. Open Session Management Panel, press ↑↓ keys, selection moves between sessions
  #   2. Open Session Management Panel with pending_merge session, press M, inline confirmation prompt appears, press Y, session merged
  #   3. Open Session Management Panel with session, press D, inline confirmation prompt appears, press Y, session discarded
  #   4. Open Session Management Panel, press Esc, panel closes and returns to AgentView
  #
  # ========================================
  Background: User Story
    As a developer using isolated sessions
    I want the Session Management Panel to respond to keyboard shortcuts
    So that I can efficiently manage my isolated sessions

  @tui
  Scenario: Navigate sessions with arrow keys
    Given the Session Management Panel is open as a full-screen view
    And there are multiple isolated sessions listed
    When I press the down arrow key
    Then the selection should move to the next session
    When I press the up arrow key
    Then the selection should move to the previous session

  @tui
  Scenario: Merge session with M key
    Given the Session Management Panel is open as a full-screen view
    And there is a session with status "pending_merge"
    And the session is selected
    When I press the "M" key
    Then an inline confirmation prompt should appear at the bottom
    When I press "Y" to confirm
    Then the session changes should be applied to the main worktree
    And the session should be removed from the list

  @tui
  Scenario: Discard session with D key
    Given the Session Management Panel is open as a full-screen view
    And there is a session selected
    When I press the "D" key
    Then an inline confirmation prompt should appear at the bottom
    When I press "Y" to confirm
    Then the worktree should be removed
    And no changes should be applied to the main worktree
    And the session should be removed from the list

  @tui
  Scenario: Close panel with Escape key
    Given the Session Management Panel is open as a full-screen view
    When I press the Escape key
    Then the panel should close
    And I should return to the AgentView
