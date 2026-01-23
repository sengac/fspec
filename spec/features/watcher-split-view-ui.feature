@watcher
@tui
@WATCH-010
Feature: Watcher Split View UI

  """
  Architecture notes:
  - Split view is implemented as a mode within AgentView, triggered when sessionGetParent(sessionId) returns non-null
  - Parent pane subscribes to parent session output via sessionGetMergedOutput and sessionAttach
  - Watcher pane uses standard AgentView conversation rendering
  - Pane state managed via useState hooks: activePane, parentConversation, watcherConversation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When viewing a watcher session, AgentView renders in split-view mode with two vertical panes
  #   2. Left pane displays parent session conversation (read-only, scrollable, dimmed styling)
  #   3. Right pane displays watcher's own conversation (interactive, full styling)
  #   4. Single input area at bottom always sends to watcher session (not parent)
  #   5. Left/Right arrow keys switch active pane (parent ↔ watcher)
  #   6. Active pane has bright styling; inactive pane has dimmed styling
  #   7. Tab key toggles turn-select mode in the active pane
  #   8. Up/Down arrow keys navigate within the active pane when in select mode
  #   9. Enter key on selected message pre-fills input with context (Discuss Selected feature)
  #   10. Parent pane content is loaded from sessionGetMergedOutput(parentSessionId) NAPI call
  #   11. Split view detects watcher session via sessionGetParent(sessionId) returning non-null
  #   12. Header shows watcher role and parent session name (e.g., '👁️ Security Reviewer (watching: Main Dev Session)')
  #
  # EXAMPLES:
  #   1. User switches to watcher session via /watcher overlay → AgentView renders with left pane showing parent conversation and right pane showing watcher conversation
  #   2. User presses Right arrow when left (parent) pane is active → right (watcher) pane becomes active with bright styling
  #   3. User presses Left arrow when right (watcher) pane is active → left (parent) pane becomes active with bright styling
  #   4. User presses Tab in active pane → turn-select mode toggles on, showing selection highlight
  #   5. User presses Up/Down in select mode → selection moves to previous/next turn in the active pane
  #   6. User selects message in parent pane and presses Enter → input is pre-filled with 'Regarding turn N in parent session: [content]...'
  #   7. User types message in input → message is sent to watcher session only (not parent)
  #   8. Regular session (non-watcher) opened → normal single-pane AgentView renders (no split view)
  #
  # ========================================

  Background: User Story
    As a user viewing a watcher session
    I want to see a split view with parent observation on the left and watcher conversation on the right
    So that I can observe the parent session while interacting with my watcher AI

  @critical
  Scenario: Split view renders when viewing a watcher session
    Given a parent session "Main Dev Session" exists with conversation history
    And a watcher session "Security Reviewer" is watching "Main Dev Session"
    When I switch to the watcher session "Security Reviewer"
    Then the view renders with two vertical panes
    And the left pane shows the parent conversation from "Main Dev Session"
    And the right pane shows the watcher conversation
    And the header shows "👁️ Security Reviewer (watching: Main Dev Session)"

  Scenario: Switch active pane to watcher with right arrow
    Given I am viewing the watcher split view
    And the left (parent) pane is currently active
    When I press the Right arrow key
    Then the right (watcher) pane becomes active
    And the watcher pane has bright styling
    And the parent pane has dimmed styling

  Scenario: Switch active pane to parent with left arrow
    Given I am viewing the watcher split view
    And the right (watcher) pane is currently active
    When I press the Left arrow key
    Then the left (parent) pane becomes active
    And the parent pane has bright styling
    And the watcher pane has dimmed styling

  Scenario: Toggle turn-select mode with Tab
    Given I am viewing the watcher split view
    And the right (watcher) pane is currently active
    When I press the Tab key
    Then turn-select mode is enabled
    And a selection highlight appears in the watcher pane

  Scenario: Navigate turns with Up/Down in select mode
    Given I am viewing the watcher split view
    And the right (watcher) pane is currently active
    And turn-select mode is enabled
    And multiple turns exist in the watcher pane
    When I press the Down arrow key
    Then the selection moves to the next turn
    When I press the Up arrow key
    Then the selection moves to the previous turn

  Scenario: Discuss selected message from parent pane
    Given I am viewing the watcher split view
    And the left (parent) pane is currently active
    And turn-select mode is enabled
    And I have selected turn 3 with content "Write a login function"
    When I press the Enter key
    Then the input is pre-filled with context from the selected turn
    And the pre-fill includes "Regarding turn 3 in parent session:"

  Scenario: Input always sends to watcher session
    Given I am viewing the watcher split view
    And the watcher session is "Security Reviewer"
    When I type "Also check for XSS vulnerabilities" in the input
    And I press Enter to send
    Then the message is sent to the watcher session
    And the message is not sent to the parent session

  Scenario: Regular session shows normal single-pane view
    Given a regular session "Dev Session" exists
    And the session has no parent (not a watcher)
    When I switch to "Dev Session"
    Then the normal single-pane AgentView renders
    And no split view is shown
