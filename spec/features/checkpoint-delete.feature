@done
@tui
@diff-viewer
@RPC-366
Feature: Checkpoint delete actions (single / all) with typed-confirmation dialog
  """
  Rust ratatui CheckpointsView (RPC-364) gains delete actions reusing the RPC-365 modal sub-state pattern. d/D opens a yes/no single-delete confirmation; a/A opens a typed-confirmation requiring the exact phrase DELETE ALL before Enter dispatches. Keys emit Action::DeleteCheckpoint / Action::DeleteAllCheckpoints which App::dispatch_checkpoints spawns onto transport delete_checkpoint / delete_all_checkpoints (RPC-362), folding a DeleteCheckpointResult back. On a successful single delete the view removes the row, clamps the selection and reloads the now-selected checkpoint's files (or CloseCheckpointsView when the list empties); delete-all closes back to the board. Any successful delete refreshes the board checkpoint_counts. Cancelling makes no transport call.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing d/D deletes the selected single checkpoint behind a yes/no confirmation when checkpoints exist
  #   2. Pressing a/A deletes all checkpoints behind a typed confirmation that requires the exact phrase DELETE ALL
  #   3. Confirming single delete dispatches delete_checkpoint, removes the row, clamps the selection and reloads the now-selected checkpoint's files and diff
  #   4. Deleting the last remaining checkpoint returns the view to the board; confirming delete-all dispatches delete_all_checkpoints and returns to the board
  #   5. Cancelling either delete dialog makes no transport call and leaves the list unchanged; after any successful delete the board checkpoint counts refresh
  #
  # EXAMPLES:
  #   1. With two checkpoints, pressing d opens a yes/no confirmation naming the selected checkpoint
  #   2. Confirming single delete of one of two checkpoints leaves one checkpoint and clamps the selection to it
  #   3. Confirming delete of the only checkpoint returns the view to the board
  #   4. Pressing a opens a typed-confirm dialog; confirm stays disabled until DELETE ALL is typed exactly, then dispatches delete_all_checkpoints and returns to the board
  #   5. Cancelling the delete-all dialog makes no transport call and leaves all checkpoints in the list
  #
  # ========================================
  Background: User Story
    As a fspec user browsing checkpoints
    I want to delete a single checkpoint or all checkpoints from the viewer behind confirmation dialogs
    So that I can prune saved checkpoints safely without leaving the TUI

  Scenario: Pressing d with checkpoints present opens a single-delete confirmation naming the checkpoint
    Given a Checkpoints view with two checkpoints
    When the user presses the d key
    Then a confirmation dialog is open naming the selected checkpoint
    And no delete action has been emitted

  Scenario: Confirming a single delete emits the DeleteCheckpoint action
    Given a single-delete confirmation dialog open for the selected checkpoint
    When the user presses the y key
    Then the view emits Action::DeleteCheckpoint for the selected checkpoint
    And the dialog shows a deleting status

  Scenario: Cancelling the single-delete confirmation closes the dialog and emits nothing
    Given a single-delete confirmation dialog open for the selected checkpoint
    When the user presses the n key
    Then no delete dialog is open
    And no delete action has been emitted

  Scenario: A single-delete result removes the row clamps the selection and reloads the new selection
    Given a Checkpoints view that has dispatched a single delete of the last of two checkpoints
    When the delete completes successfully
    Then the deleted checkpoint is removed leaving one checkpoint
    And the selection is clamped to the remaining checkpoint
    And the view emits Action::LoadCheckpointFiles for the remaining checkpoint
    And the view emits Action::RefreshCheckpointCounts

  Scenario: Deleting the only remaining checkpoint returns the view to the board
    Given a Checkpoints view that has dispatched a single delete of its only checkpoint
    When the delete completes successfully
    Then the checkpoint list is empty
    And the view emits Action::CloseCheckpointsView
    And the view emits Action::RefreshCheckpointCounts

  Scenario: Pressing a opens a typed-confirmation that stays disabled until DELETE ALL is typed exactly
    Given a Checkpoints view with two checkpoints
    When the user presses the a key
    Then a typed-confirmation dialog is open requiring the phrase DELETE ALL
    And the confirmation is disabled until DELETE ALL is typed exactly
    And no delete action has been emitted

  Scenario: Pressing Enter before DELETE ALL is typed exactly does not dispatch
    Given a typed-confirmation delete-all dialog with the partial text DELETE typed
    When the user presses the Enter key
    Then no delete action has been emitted
    And the typed-confirmation dialog is still open

  Scenario: Confirming the typed delete-all dispatches DeleteAllCheckpoints
    Given a typed-confirmation delete-all dialog with DELETE ALL typed exactly
    When the user presses the Enter key
    Then the view emits Action::DeleteAllCheckpoints
    And the dialog shows a deleting status

  Scenario: Cancelling the delete-all dialog makes no transport call and leaves the list unchanged
    Given a typed-confirmation delete-all dialog is open with two checkpoints present
    When the user presses the Esc key
    Then no delete dialog is open
    And no delete action has been emitted
    And the checkpoint list still has two checkpoints
