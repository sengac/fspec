@done
@diff-viewer
@tui
@RPC-365
Feature: Checkpoint restore actions (single file / all files) with confirmation and progress dialog
  """
  Confirming a restore re-requests the file diff and refreshes the board counts
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing r/R restores the selected single file but only when the Files pane is focused and the checkpoint has files; otherwise it is a no-op
  #   2. Pressing t/T restores all files of the selected checkpoint when checkpoints exist
  #   3. A restore key opens a confirmation dialog (overwrite warning) and only dispatches the transport restore call on confirm; cancelling makes no call
  #   4. After confirm a status dialog shows restoring then complete on success or error with the message on failure
  #   5. After a successful single-file restore the diff pane reloads for that file; after any successful restore the board checkpoint counts refresh
  #
  # EXAMPLES:
  #   1. Files pane focused with a.txt selected: pressing r opens a single-file restore confirmation naming a.txt
  #   2. Checkpoints pane focused: pressing r does nothing (no dialog opens) because the Files pane is not focused
  #   3. Pressing t opens a restore-all confirmation naming the number of files in the selected checkpoint
  #   4. Confirming a single-file restore dispatches restore_checkpoint_file and the status dialog shows complete on success
  #   5. Cancelling the restore confirmation closes the dialog and makes no transport call
  #
  # ========================================
  Background: User Story
    As a fspec user browsing checkpoints
    I want to restore a single file or all files of a checkpoint with a confirmation and progress dialog
    So that I can roll back to a saved checkpoint safely from the TUI

  Scenario: Pressing r with the Files pane focused opens a single-file restore confirmation naming the file
    Given a Checkpoints view with the Files pane focused and a.txt selected
    When the user presses the r key
    Then a confirmation dialog is open naming a.txt
    And no restore action has been emitted

  Scenario: Pressing r with the Checkpoints pane focused is a no-op
    Given a Checkpoints view with the Checkpoints pane focused and the selected checkpoint has files
    When the user presses the r key
    Then no confirmation dialog is open
    And no restore action has been emitted

  Scenario: Pressing t opens a restore-all confirmation naming the file count
    Given a Checkpoints view whose selected checkpoint changed two files
    When the user presses the t key
    Then a confirmation dialog is open naming 2 files
    And no restore action has been emitted

  Scenario: Confirming a single-file restore emits the RestoreCheckpointFile action
    Given a confirmation dialog open for restoring the single file a.txt
    When the user presses the y key
    Then the view emits Action::RestoreCheckpointFile for a.txt
    And the dialog shows a restoring status

  Scenario: Cancelling the restore confirmation closes the dialog and emits nothing
    Given a confirmation dialog open for restoring the single file a.txt
    When the user presses the n key
    Then no confirmation dialog is open
    And no restore action has been emitted

  Scenario: A restore result drives the status dialog to complete then refreshes the diff and counts
    Given a Checkpoints view that has dispatched a single-file restore of a.txt
    When the restore completes successfully
    Then the dialog shows a complete status
    And the view emits Action::LoadCheckpointFileDiff for a.txt
    And the view emits Action::RefreshCheckpointCounts

  Scenario: A failed restore drives the status dialog to error with the message
    Given a Checkpoints view that has dispatched a single-file restore of a.txt
    When the restore fails with the message disk full
    Then the dialog shows an error status containing disk full
    And no diff reload action has been emitted
