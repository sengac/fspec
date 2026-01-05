@done
@confirmation
@destructive-action
@ink
@react
@modal
@session
@tui
@TUI-040
Feature: Delete Session from Resume View with Confirmation Dialog
  """
  Modifies AgentView.tsx resume mode: adds D key handler, showDeleteDialog state, and ThreeButtonDialog rendering
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Resume view currently has read-only functionality - no delete operations exist
  #   2. Session deletion must call persistenceDeleteSession from codelet-napi to remove session file and update storage
  #   3. After successful deletion, session list must be refreshed and selection index adjusted
  #   4. If all sessions are deleted, resume mode should exit automatically
  #   5. ESC key cancels the delete dialog and returns to normal resume mode state
  #   6. Footer must display available keybindings including delete options
  #   7. Delete dialog uses existing ConfirmationDialog component for consistency with CheckpointViewer pattern
  #   8. D key opens a three-button confirmation dialog with options: Delete This Session, Delete ALL Sessions, Cancel
  #   9. Three-button dialog requires a new component or extended ConfirmationDialog since current component only supports Y/N binary confirmation
  #   10. After deleting session(s), trigger cleanup of orphaned messages in the message store
  #   11. Three-button dialog navigation: Left/Right arrow keys to navigate between buttons, Enter to select, ESC to cancel
  #
  # EXAMPLES:
  #   1. User deletes the last remaining session, session is deleted, resume mode automatically exits since no sessions remain
  #   2. User presses ESC while delete ALL dialog is showing, deletion is cancelled, all sessions remain intact
  #   3. User selects a session, presses D, three-button dialog appears with 'Delete This Session', 'Delete ALL Sessions', 'Cancel' options
  #   4. User presses D, dialog appears, user selects 'Delete This Session', the selected session is deleted, list refreshes with next session selected
  #   5. User presses D, dialog appears, user selects 'Delete ALL Sessions', all sessions are deleted, resume mode exits
  #   6. User sees footer in resume mode showing 'Enter Select | ↑↓ Navigate | D Delete | Esc Cancel'
  #   7. User presses D, dialog appears with 'Delete This Session' highlighted, user presses Right arrow to highlight 'Delete ALL Sessions', presses Enter to confirm
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should deleting a session also trigger any cleanup of orphaned messages in the message store, or is that handled separately?
  #   A: Arrow keys to navigate between buttons + Enter to select (Option A)
  #
  #   Q: For the keybinding, should we use Shift+D or A for delete all? The CheckpointViewer uses A for delete all.
  #   A: Use single D key which opens a three-button dialog: Delete This Session, Delete ALL Sessions, Cancel - not separate keybindings
  #
  #   Q: How should users navigate and select options in the three-button dialog? Options: (A) Arrow keys to navigate between buttons + Enter to select, (B) Number keys 1/2/3 to directly select, (C) Letter keys like D/A/C for Delete/All/Cancel?
  #   A: Arrow keys to navigate between buttons + Enter to select (Option A)
  #
  # ========================================
  Background: User Story
    As a TUI user managing agent sessions
    I want to delete unwanted sessions from the /resume view
    So that I can clean up old sessions and maintain a tidy session history

  Scenario: D key opens three-button delete confirmation dialog
    Given I am in resume mode with sessions available
    When I press the D key
    Then a three-button dialog appears with options Delete This Session, Delete ALL Sessions, and Cancel

  Scenario: Cancel option closes dialog without deleting
    Given I am in resume mode with the delete confirmation dialog open
    When I navigate to Cancel and press Enter
    Then the dialog closes
    And no sessions are deleted
    And resume mode remains active

  Scenario: ESC key cancels delete dialog
    Given I am in resume mode with the delete confirmation dialog open
    When I press the ESC key
    Then the dialog closes
    And no sessions are deleted

  Scenario: Resume mode footer displays delete keybinding
    Given I am in resume mode
    When I view the footer
    Then I see the D Delete keybinding displayed

  Scenario: Session deletion triggers orphaned message cleanup
    Given I am in resume mode with sessions that have associated messages
    When I delete a session
    Then orphaned messages in the message store are cleaned up

  Scenario: Delete This Session removes selected session and refreshes list
    Given I am in resume mode with multiple sessions and the delete confirmation dialog is open
    When I select Delete This Session and press Enter
    Then the selected session is deleted
    And the session list refreshes with the next session selected
    And the dialog closes

  Scenario: Delete ALL Sessions removes all sessions and exits resume mode
    Given I am in resume mode with multiple sessions and the delete confirmation dialog is open
    When I navigate to Delete ALL Sessions and press Enter
    Then all sessions are deleted
    And resume mode exits automatically

  Scenario: Arrow keys navigate between dialog buttons
    Given I am in resume mode with the delete confirmation dialog open and Delete This Session is highlighted
    When I press the Right arrow key
    Then Delete ALL Sessions becomes highlighted

  Scenario: Deleting last session exits resume mode
    Given I am in resume mode with only one session and the delete confirmation dialog is open
    When I select Delete This Session and press Enter
    Then the session is deleted
    And resume mode exits automatically since no sessions remain
