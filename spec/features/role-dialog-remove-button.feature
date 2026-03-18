@TUI-082
Feature: Remove Role button on RoleBanner for quick role clearing
  """
  Modify RoleDialog.tsx to accept a showRemove prop (true when initialRole is non-empty). Add 'remove' to FocusArea type. Tab cycle and arrow navigation updated to include 'remove' when visible. Remove button calls onSubmit('') to trigger the BUG-121-fixed clear path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After removal, the RoleBanner disappears (zero height) and a status message 'Role cleared.' appears in the conversation
  #   2. No confirmation dialog. Instead, add a 'Remove' button as a third button in the /role dialog between OK and Cancel. The Remove button only appears when an existing role is set.
  #   3. The /role dialog shows 3 buttons when an existing role is set: OK | Remove | Cancel
  #   4. The /role dialog shows 2 buttons when no role exists: OK | Cancel (no Remove button)
  #   5. Tab cycles focus: textarea → OK → Remove → Cancel → textarea (when Remove is visible)
  #   6. Pressing Enter on the Remove button clears the role (calls sessionSetRole with empty string) and closes the dialog
  #   7. Left/right arrows navigate between all visible buttons (OK, Remove, Cancel)
  #   8. The Remove button uses red color styling to distinguish it as a destructive action
  #
  # EXAMPLES:
  #   1. Session has role 'security reviewer' → user types /role → dialog opens with 'security reviewer' pre-populated and 3 buttons: OK | Remove | Cancel → user presses Tab twice to reach Remove → presses Enter → role cleared → dialog closes → RoleBanner disappears
  #   2. Session has no role → user types /role → dialog opens with empty textarea and 2 buttons: OK | Cancel (no Remove button)
  #   3. Session has role 'architect' → user types /role → dialog opens with 3 buttons → user presses Tab to OK → right arrow to Remove (red highlight) → right arrow to Cancel → left arrow back to Remove → confirms button navigation works across all three
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should pressing 'x' require a confirmation dialog, or is instant removal acceptable since /role can restore it quickly?
  #   A: No confirmation dialog. Instead, add a 'Remove' button as a third button in the /role dialog between OK and Cancel. The Remove button only appears when an existing role is set.
  #
  # ========================================
  Background: User Story
    As a user with an active session role
    I want to remove the role via a Remove button in the /role dialog
    So that I can clear the role without manually deleting the text

  Scenario: Remove button appears when editing an existing role
    Given a session exists with role "security reviewer"
    When the user opens the /role dialog
    Then the dialog shows 3 buttons: OK, Remove, and Cancel
    And the Remove button is styled in red

  Scenario: Remove button is hidden when creating a new role
    Given a session exists with no role
    When the user opens the /role dialog
    Then the dialog shows 2 buttons: OK and Cancel
    And no Remove button is visible

  Scenario: Pressing Remove clears the role and closes the dialog
    Given a session exists with role "security reviewer"
    And the user has opened the /role dialog
    When the user navigates to the Remove button and presses Enter
    Then the role should be cleared
    And the dialog should close
    And the RoleBanner should not be visible

  Scenario: Tab cycles through all three buttons when Remove is visible
    Given a session exists with role "architect"
    And the user has opened the /role dialog
    When the user presses Tab from the textarea
    Then focus moves to OK
    When the user presses Tab again
    Then focus moves to Remove
    When the user presses Tab again
    Then focus moves to Cancel
    When the user presses Tab again
    Then focus returns to the textarea

  Scenario: Left/right arrows navigate between all visible buttons
    Given a session exists with role "architect"
    And the user has opened the /role dialog
    And focus is on the OK button
    When the user presses the right arrow key
    Then focus moves to Remove
    When the user presses the right arrow key
    Then focus moves to Cancel
    When the user presses the left arrow key
    Then focus moves to Remove
