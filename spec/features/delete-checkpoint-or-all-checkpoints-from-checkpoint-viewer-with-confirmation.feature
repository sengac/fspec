@checkpoints
@critical
@tui
@git
@checkpoint
@modal
@react
@ink
@confirmation
@destructive-action
@GIT-010
Feature: Delete checkpoint or all checkpoints from checkpoint viewer with confirmation
  """
  Backend: Add deleteCheckpoint() and deleteAllCheckpoints() functions to git-checkpoint.ts using isomorphic-git to delete refs and update index files. Frontend: Integrate ConfirmationDialog component into CheckpointViewer.tsx with D/Shift+D key handlers. Uses composition pattern: ConfirmationDialog wraps Dialog for modal rendering. Deletion updates CheckpointPanel via IPC (sendIPCMessage). Risk-aware UX: medium risk (yellow) for single deletion, high risk (red) for delete all.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint viewer currently has read-only functionality - no delete operations exist in the TUI
  #   2. cleanup-checkpoints CLI command exists but only keeps last N checkpoints, doesn't allow selective deletion
  #   3. Checkpoint deletion must remove both the git ref (refs/fspec-checkpoints/{workUnitId}/{checkpointName}) and the index file entry
  #   4. User wants confirmation dialogs to prevent accidental deletion
  #   5. Delete operation must remove both the git ref (refs/fspec-checkpoints/{workUnitId}/{checkpointName}) AND the checkpoint entry from the index file (.git/fspec-checkpoints-index/{workUnitId}.json)
  #   6. Single checkpoint deletion uses D key and ConfirmationDialog with confirmMode='yesno' and riskLevel='medium' (yellow border)
  #   7. Delete ALL uses Shift+D key and ConfirmationDialog with confirmMode='typed', typedPhrase='DELETE ALL', riskLevel='high' (red border)
  #   8. After successful deletion, viewer stays open with next checkpoint selected (or exits to board if no checkpoints remain)
  #   9. Deletion triggers IPC message (sendIPCMessage({ type: 'checkpoint-changed' })) to update CheckpointPanel counts in real-time
  #
  # EXAMPLES:
  #   1. User navigates to checkpoint viewer, selects an old checkpoint they don't need, presses Delete key, sees confirmation dialog, confirms, checkpoint is deleted
  #   2. User has 50 old checkpoints, wants to delete them all at once, presses key combination, sees 'Delete ALL checkpoints?' warning, confirms with typed confirmation, all deleted
  #   3. User presses D key while viewing checkpoint, yellow dialog appears asking 'Delete checkpoint AUTH-001-baseline?', user presses Y, checkpoint deleted, viewer selects next checkpoint
  #   4. User presses D key, sees yellow dialog, presses N, deletion cancelled, viewer returns to normal state
  #   5. User presses Shift+D, red dialog appears 'Delete ALL 47 checkpoints for AUTH-001?', user types 'DELETE ALL' and presses Enter, all checkpoints deleted, viewer exits to board
  #   6. User deletes last remaining checkpoint in viewer, viewer automatically exits to board (empty state)
  #
  # QUESTIONS (ANSWERED):
  #   Q: What key should trigger single checkpoint deletion? (D)elete, (X) remove, (Backspace), or other?
  #   A: true
  #
  #   Q: What key/combination should trigger delete ALL? Shift+D, Ctrl+D, separate menu option, or other?
  #   A: true
  #
  #   Q: For single checkpoint deletion, what should the confirmation look like? Simple Y/N prompt, or require typing checkpoint name, or other?
  #   A: true
  #
  #   Q: For delete ALL confirmation, should it require typing a specific phrase like DELETE ALL to prevent accidents?
  #   A: true
  #
  #   Q: After deleting a checkpoint, what should happen? Stay in viewer with next checkpoint selected, exit viewer, show success message, or other?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a TUI developer managing checkpoints
    I want to delete unwanted checkpoints from the checkpoint viewer
    So that I can clean up old experiments and maintain a tidy checkpoint history

  Scenario: Delete single checkpoint with D key and Y/N confirmation
    Given I am in the checkpoint viewer
    And there is a checkpoint named "AUTH-001-baseline" selected
    When I press the D key
    Then a yellow confirmation dialog should appear with message "Delete checkpoint 'AUTH-001-baseline'?"
    And the dialog should use yesno confirmation mode
    And the dialog should have medium risk level
    When I press the Y key
    Then the checkpoint "AUTH-001-baseline" should be deleted
    And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-baseline" should be removed
    And the checkpoint entry should be removed from the index file
    And the viewer should select the next checkpoint in the list
    And an IPC message with type "checkpoint-changed" should be sent

  Scenario: Cancel single checkpoint deletion with N key
    Given I am in the checkpoint viewer
    And there is a checkpoint named "AUTH-001-experiment" selected
    When I press the D key
    Then a yellow confirmation dialog should appear
    When I press the N key
    Then the deletion should be cancelled
    And the checkpoint "AUTH-001-experiment" should still exist
    And the viewer should return to normal state

  Scenario: Delete ALL checkpoints with Shift+D and typed confirmation
    Given I am in the checkpoint viewer
    And there are 47 checkpoints for work unit "AUTH-001"
    When I press Shift+D
    Then a red confirmation dialog should appear with message "Delete ALL 47 checkpoints for AUTH-001?"
    And the dialog should use typed confirmation mode
    And the dialog should require typing "DELETE ALL"
    And the dialog should have high risk level
    When I type "DELETE ALL" and press Enter
    Then all 47 checkpoints for "AUTH-001" should be deleted
    And all git refs under "refs/fspec-checkpoints/AUTH-001/" should be removed
    And the index file for "AUTH-001" should be deleted
    And the viewer should exit to the board
    And an IPC message with type "checkpoint-changed" should be sent

  Scenario: Delete last remaining checkpoint exits to board
    Given I am in the checkpoint viewer
    And there is only 1 checkpoint remaining
    When I press the D key
    And I press the Y key to confirm
    Then the last checkpoint should be deleted
    And the viewer should automatically exit to the board

  Scenario: Cancel delete ALL with ESC key
    Given I am in the checkpoint viewer
    And there are 50 checkpoints for work unit "TUI-001"
    When I press Shift+D
    Then a red confirmation dialog should appear
    When I press the ESC key
    Then the deletion should be cancelled
    And all 50 checkpoints should still exist
    And the viewer should return to normal state
