@done
@WATCH-016
Feature: Discuss Selected Feature

  """
  Architecture notes:
  - Enter behavior is implemented in SplitSessionView.tsx useInput handler
  - Parent pane Enter uses generateDiscussSelectedPrefill from turnSelection.ts
  - Watcher pane Enter opens TurnContentModal by setting showTurnContent state and selectedTurnContent
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Enter key on a selected turn in the parent pane pre-fills the watcher input with 'Regarding turn N in parent session:' followed by a code-fenced preview of the turn content
  #   2. Enter key on a selected turn in the watcher pane opens the TurnContentModal to show full turn content (existing behavior from TUI-045)
  #   3. After pre-filling input from parent pane, select mode is exited and cursor is positioned after the pre-fill for user to type their question
  #   4. The turn content preview is truncated to 50 characters with '...' suffix if longer
  #   5. Enter key behavior only applies when turn-select mode is active (Tab to toggle)
  #
  # EXAMPLES:
  #   1. User selects turn 3 in parent pane showing 'Write a login function', presses Enter → input is pre-filled with 'Regarding turn 3 in parent session:\n\`\`\`\nWrite a login function\n\`\`\`\n' → user can then type 'Is this secure?' and send to watcher
  #   2. User selects turn 2 in watcher pane showing a long SQL injection warning, presses Enter → TurnContentModal opens showing full watcher response with proper scrolling
  #   3. User presses Enter on a selected turn in parent pane with very long content (200+ chars) → preview shows first 50 chars + '...' to avoid cluttering the input area
  #   4. User presses Enter on a selected turn in parent pane → select mode exits → input area gains focus → user sees pre-filled context and blinking cursor ready for input
  #   5. User presses Enter on watcher pane selection while TurnContentModal is already showing → modal updates to show the newly selected turn content
  #
  # ========================================

  Background: User Story
    As a user viewing a watcher session in split view
    I want to press Enter on a selected message to discuss it or view its full content
    So that I can either discuss parent content with my watcher (parent pane) or view full watcher turn content (watcher pane)

  @critical
  Scenario: Enter on selected turn in parent pane pre-fills input with context
    Given I am viewing a watcher session in split view
    And the parent pane is active with turn-select mode enabled
    And turn 3 is selected with content "Write a login function"
    When I press the Enter key
    Then the input area is pre-filled with "Regarding turn 3 in parent session:"
    And the pre-fill includes a code-fenced preview of the turn content
    And turn-select mode is exited
    And the cursor is positioned after the pre-fill for typing

  Scenario: Enter on selected turn in watcher pane opens full content modal
    Given I am viewing a watcher session in split view
    And the watcher pane is active with turn-select mode enabled
    And turn 2 is selected with a long SQL injection warning message
    When I press the Enter key
    Then the TurnContentModal opens
    And the modal shows the full watcher response with scrolling support

  Scenario: Long content in parent pane is truncated in pre-fill
    Given I am viewing a watcher session in split view
    And the parent pane is active with turn-select mode enabled
    And a turn is selected with content exceeding 50 characters
    When I press the Enter key
    Then the pre-fill shows only the first 50 characters
    And the preview ends with "..." to indicate truncation

  Scenario: Select mode exits after discussing parent turn
    Given I am viewing a watcher session in split view
    And the parent pane is active with turn-select mode enabled
    And a turn is selected in the parent pane
    When I press the Enter key
    Then turn-select mode is disabled
    And the input area gains focus
    And the user can type their question after the pre-fill

  Scenario: Modal updates when selecting different watcher turn
    Given I am viewing a watcher session in split view
    And the watcher pane is active with turn-select mode enabled
    And the TurnContentModal is already open showing turn 1
    And I navigate to select turn 2 in the watcher pane
    When I press the Enter key
    Then the TurnContentModal updates to show turn 2 content
