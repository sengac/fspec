@feature-management
@cli
@done
@session-management
@tui
@GIT-030
Feature: Enter key on board bypasses Create Session dialog - no isolated option
  """
  BoardView onEnter handler must call openCreateSessionDialog() instead of navigateToNewSession() when no attached session exists. The CreateSessionDialog shows 3 options: Yes, Yes - Isolated, Cancel (TUI-090). Integration point: BoardView.tsx onEnter callback.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When user presses Enter on a work unit with no attached session, the Create Session dialog must be shown
  #   2. When user presses Enter on a work unit with an attached session, navigate directly to that session (no dialog)
  #   3. The Create Session dialog must show 3 options: Yes, Yes - Isolated, Cancel (TUI-090)
  #
  # EXAMPLES:
  #   1. User selects work unit AUTH-001 with no attached session, presses Enter, Create Session dialog appears with Yes/Yes-Isolated/Cancel options
  #   2. User selects work unit AUTH-001 with attached session abc-123, presses Enter, navigates directly to session abc-123 (no dialog)
  #   3. User presses Enter on work unit, sees dialog, selects Isolated, confirms, isolated session is created
  #
  # ========================================
  Background: User Story
    As a user
    I want to press Enter on a work unit from the board
    So that choose whether to create a normal or isolated session

  Scenario: Show Create Session dialog when no attached session
    Given I am viewing the board with a work unit that has no attached session
    When I select the work unit and press Enter
    Then the Create Session dialog should appear with Yes, Yes - Isolated, and Cancel options

  Scenario: Navigate directly to attached session
    Given I am viewing the board with a work unit that has attached session abc-123
    When I select the work unit and press Enter
    Then I should navigate to session abc-123 without seeing the Create Session dialog
