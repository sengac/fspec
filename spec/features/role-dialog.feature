@done
@tui-component
@AMGR-012
Feature: Role dialog — /role TUI command for session role management

  """
  RoleDialog component at src/components/RoleDialog.tsx — wraps base Dialog, uses useMultiLineInput hook for text editing, implements Tab-based focus cycling (textarea → OK → Cancel → textarea)
  /role slash command integrated in AgentView.tsx following /thinking pattern — checks currentSessionId, opens dialog or shows status message
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /role command opens a modal dialog with a multi-line text area (6 fixed visible lines) pre-populated with the current role text if one exists
  #   2. Dialog has OK/Cancel buttons at the bottom — Tab cycles focus between the text area and the button row, left/right arrows navigate between OK and Cancel when buttons are focused
  #   3. Enter key inserts a newline in the text area (when text area is focused). When button row is focused, Enter activates the selected button
  #   4. Submitting with an empty text area clears/removes the role from the session
  #   5. ESC dismisses the dialog without making any changes (handled by base Dialog component)
  #   6. Dialog uses cyan border color and the base Dialog component from src/components/Dialog.tsx
  #   7. /role requires an active session — shows status message if no session exists
  #
  # EXAMPLES:
  #   1. User types /role → dialog opens with empty textarea → types 'code-reviewer' → presses Tab → focus moves to OK button → presses Enter → role is set to 'code-reviewer'
  #   2. Session has role 'architect' → user types /role → dialog opens with 'architect' pre-populated in textarea → user edits to 'senior architect' → presses Tab then Enter on OK → role updated to 'senior architect'
  #   3. Session has role 'reviewer' → user types /role → dialog opens with 'reviewer' → user clears the text area → presses Tab then Enter on OK → role is removed/cleared
  #   4. User types /role → dialog opens → user presses ESC → dialog closes, role unchanged
  #   5. User types /role → dialog opens → user presses Tab → OK button highlighted → presses Tab again → Cancel button highlighted → presses Tab → focus back to textarea
  #   6. User types /role with no active session → status message 'Start a session first to set a role.' displayed, no dialog
  #   7. In the dialog text area: cursor is visible, Enter creates newline, arrow keys navigate, backspace deletes — standard multi-line editing using useMultiLineInput hook
  #
  # ========================================

  Background: User Story
    As a user
    I want to set or edit a role on any session via a /role TUI dialog
    So that I can customize the system prompt overlay for sessions to specialize their behavior

  Scenario: Set role via /role dialog on session with no existing role
    Given I have an active session with no role set
    When I type "/role"
    Then a role dialog opens with an empty text area of 6 visible lines
    And the dialog has a cyan border
    And the text area is focused
    When I type "code-reviewer" in the text area
    And I press Tab to move focus to the button row
    Then the OK button is highlighted
    When I press Enter
    Then the dialog closes
    And the session role is set to "code-reviewer"

  Scenario: Edit existing role via /role dialog
    Given I have an active session with role "architect"
    When I type "/role"
    Then a role dialog opens with "architect" pre-populated in the text area
    When I clear the text area and type "senior architect"
    And I press Tab to move focus to the button row
    And I press Enter on the OK button
    Then the dialog closes
    And the session role is updated to "senior architect"

  Scenario: Clear role by submitting empty text area
    Given I have an active session with role "reviewer"
    When I type "/role"
    Then a role dialog opens with "reviewer" pre-populated in the text area
    When I clear the text area completely
    And I press Tab to move focus to the button row
    And I press Enter on the OK button
    Then the dialog closes
    And the session role is cleared

  Scenario: Cancel role dialog with ESC
    Given I have an active session with role "tester"
    When I type "/role"
    Then a role dialog opens with "tester" pre-populated in the text area
    When I press ESC
    Then the dialog closes without changes
    And the session role remains "tester"

  Scenario: Tab cycles focus between text area and buttons
    Given I have an active session
    When I type "/role"
    Then a role dialog opens with the text area focused
    When I press Tab
    Then the OK button is highlighted
    When I press Tab again
    Then the Cancel button is highlighted
    When I press Tab again
    Then the text area is focused again

  Scenario: Cancel button dismisses dialog without changes
    Given I have an active session with role "original-role"
    When I type "/role"
    Then a role dialog opens
    When I type "new-role" in the text area
    And I press Tab twice to move focus to Cancel
    And I press Enter on the Cancel button
    Then the dialog closes without changes
    And the session role remains "original-role"

  Scenario: Left/right arrows navigate between OK and Cancel buttons
    Given I have an active session
    When I type "/role"
    And I press Tab to move focus to the button row
    Then the OK button is highlighted
    When I press the right arrow key
    Then the Cancel button is highlighted
    When I press the left arrow key
    Then the OK button is highlighted

  Scenario: /role requires an active session
    Given I have no active session
    When I type "/role"
    Then a status message "Start a session first to set a role." is displayed
    And no dialog opens

  Scenario: Multi-line text editing in role dialog
    Given I have an active session
    When I type "/role"
    Then a role dialog opens with the text area focused
    When I type "Line one" in the text area
    And I press Enter to insert a newline
    And I type "Line two"
    Then the text area contains two lines
    And the cursor is on the second line
