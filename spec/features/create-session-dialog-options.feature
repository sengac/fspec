@session-management
@tui
@done
@TUI-090
Feature: CreateSessionDialog should have 3 options: Yes, Yes - Isolated, Cancel
  """
  Replaces Yes/No buttons + Normal/Isolated toggle with 3 flat options. Component: src/components/CreateSessionDialog.tsx. Callers: BoardView.tsx, AgentView.tsx via useSessionNavigation.ts
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dialog must show exactly 3 options: Yes, Yes - Isolated, Cancel
  #   2. Left/Right arrows navigate between the 3 options cyclically
  #   3. Enter confirms the selected option: Yes calls onConfirm(false), Yes - Isolated calls onConfirm(true), Cancel calls onCancel()
  #   4. ESC always cancels regardless of selected option
  #   5. Default selected option is Yes (first option)
  #   6. Context-aware title/description (TUI-067) still applies: 'Work on ID?' for work unit, 'Start New Agent?' for unattached
  #
  # EXAMPLES:
  #   1. User opens dialog, sees [Yes] highlighted, presses Enter → normal session created
  #   2. User opens dialog, presses Right to select 'Yes - Isolated', presses Enter → isolated session created
  #   3. User opens dialog, presses Right twice to select 'Cancel', presses Enter → dialog closes, no session created
  #   4. User opens dialog, presses ESC → dialog closes regardless of which option is highlighted
  #   5. User opens dialog from board work unit AUTH-001, sees 'Work on AUTH-001?' with Yes/Yes-Isolated/Cancel options
  #
  # ========================================
  Background: User Story
    As a user
    I want to see a simple 3-option dialog when creating a new session
    So that quickly choose between normal, isolated, or cancel without navigating a complex toggle UI

  Scenario: Default option is Yes and creates normal session on Enter
    Given the Create Session dialog is open
    When I press Enter
    Then the "Yes" option should be highlighted by default
    Then onConfirm should be called with isolated=false

  Scenario: Selecting Yes - Isolated creates isolated session
    Given the Create Session dialog is open
    When I press Right to select "Yes - Isolated"
    Then onConfirm should be called with isolated=true
    And I press Enter

  Scenario: Selecting Cancel closes dialog without creating session
    Given the Create Session dialog is open
    When I press Right twice to select "Cancel"
    Then onCancel should be called
    And I press Enter
    And no session should be created

  Scenario: ESC cancels regardless of selected option
    Given the Create Session dialog is open
    When I press ESC
    Then onCancel should be called
    And "Yes - Isolated" is currently highlighted

  Scenario: Context-aware title with 3 options for work unit
    Given I am viewing the board with work unit "AUTH-001"
    When the Create Session dialog opens for that work unit
    Then the dialog title should be "Work on AUTH-001?"
    And I should see options "Yes", "Yes - Isolated", and "Cancel"
