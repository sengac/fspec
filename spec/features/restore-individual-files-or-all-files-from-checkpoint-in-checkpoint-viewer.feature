@done
@high
@tui
@checkpoint-management
@git
@GIT-009
Feature: Restore individual files or all files from checkpoint in checkpoint viewer
  """
  Keybindings: R key=restore selected file (only when files pane focused), T key=restore all files (when checkpoints/files pane focused). Delete keybindings already use D=single, A=delete all.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint viewer currently has three panes: checkpoints list (left), files in checkpoint (middle), and diff viewer (right)
  #   2. Checkpoint viewer currently only allows viewing/browsing checkpoints - no restore functionality exists
  #   3. restoreCheckpoint() function already exists in git-checkpoint.ts and handles conflict detection
  #   4. Current restoreCheckpoint() only restores ALL files at once. Single-file restore requires new utility function restoreCheckpointFile()
  #   5. Checkpoint viewer displays all checkpoints from all work units (loaded from .git/fspec-checkpoints-index/*.json), sorted by timestamp (most recent first), limited to 200 for performance
  #   6. After successful restore, IPC message {type: 'checkpoint-changed'} must be sent to notify other components (BoardView, CheckpointPanel)
  #   7. Current keybindings: D=delete single, A=delete ALL. Restore needs different keys to avoid conflicts. Must use different keybindings for restore operations.
  #   8. Files in checkpoint viewer show CHANGED files only (compared against checkpoint parent), not all files in repo
  #
  # EXAMPLES:
  #   1. User selects checkpoint 'AUTH-001-baseline' with 3 files (src/auth.ts, src/login.ts, src/utils.ts), presses R key on src/auth.ts, confirmation dialog appears (if enabled), after confirmation file is restored from checkpoint, diff pane refreshes showing new comparison
  #   2. User selects checkpoint 'TUI-001-auto-testing', presses T key (or other chosen keybinding), confirmation dialog appears 'Restore ALL 15 files from checkpoint?', user confirms, all 15 files are restored, IPC message sent, CheckpointPanel updates
  #   3. User attempts to restore src/deleted-file.ts that no longer exists in working directory, system either: (A) creates file + parent dirs, (B) shows warning + confirmation, or (C) shows error (depending on answer to Question 6)
  #   4. User restores src/config.ts which has uncommitted changes in working directory, conflict detected, confirmation dialog shows: 'Overwrite src/config.ts? Current changes will be LOST.' with high-risk warning, user must explicitly confirm
  #   5. User in files pane (middle), presses R key on selected file, restore dialog appears. User in checkpoints pane (left), presses T key, restore all dialog appears. User in diff pane (right), R/T keys do nothing (no file selected)
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should the user trigger restore? (A) Press 'R' key while file is selected to restore single file, (B) Press 'A' key to restore all files from checkpoint, (C) Show a restore menu with options, (D) other approach?
  #   A: true
  #
  #   Q: What should happen if restoring would overwrite uncommitted changes? (A) Show warning and require confirmation, (B) Block restore and show error, (C) Auto-create checkpoint before restore, (D) other approach?
  #   A: true
  #
  #   Q: After a successful restore, what should happen? (A) Show success message and stay in viewer, (B) Exit viewer and return to board, (C) Show updated diff comparing restored version vs previous, (D) other?
  #   A: true
  #
  #   Q: Should there be visual indicators showing which files have conflicts/would be overwritten before restore? (yes/no)
  #   A: true
  #
  #   Q: What keybindings should trigger restore? (A) R key for single file + T key for restore all files, (B) R key for single file + Shift+R for restore all, (C) Enter key for single file + T key for restore all, (D) other approach?
  #   A: true
  #
  #   Q: Should single-file restore show a confirmation dialog? (A) Yes, always confirm (safety first), (B) Only if conflicts detected, (C) No confirmation for single files (faster UX), (D) other approach?
  #   A: true
  #
  #   Q: When restoring a file that doesn't exist in current working directory (was deleted), should we: (A) Create the file and all parent directories automatically, (B) Show warning and require confirmation, (C) Skip the file and show error, (D) other approach?
  #   A: true
  #
  #   Q: Should the diff pane update after restore to show the NEW diff (checkpoint vs new HEAD)? (A) Yes, refresh diff automatically, (B) No, keep showing old diff, (C) Exit viewer after restore, (D) other approach?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using checkpoint viewer
    I want to restore individual files or all files from a checkpoint
    So that I can recover specific file versions without restoring the entire checkpoint

  Scenario: Restore single file from checkpoint with R key
    Given I am in the checkpoint viewer
    And a checkpoint "AUTH-001-baseline" is selected with 3 files
    And the files pane is focused with "src/auth.ts" selected
    When I press the R key
    Then a confirmation dialog should appear with message "Restore src/auth.ts from checkpoint 'AUTH-001-baseline'?"
    And the dialog should use yesno confirmation mode
    And the dialog should have medium risk level
    When I press the Y key to confirm
    Then the file "src/auth.ts" should be restored from the checkpoint
    And the diff pane should refresh showing new comparison
    And a success message should be displayed
    And the viewer should stay open
    And an IPC message with type "checkpoint-changed" should be sent

  Scenario: Restore all files from checkpoint with T key
    Given I am in the checkpoint viewer
    And a checkpoint "TUI-001-auto-testing" is selected with 15 files
    When I press the T key
    Then a confirmation dialog should appear with message "Restore ALL 15 files from checkpoint 'TUI-001-auto-testing'?"
    And the dialog should use yesno confirmation mode
    And the dialog should have high risk level
    When I press the Y key to confirm
    Then all 15 files should be restored from the checkpoint
    And the diff pane should refresh showing new comparison
    And a success message should be displayed
    And the viewer should stay open
    And an IPC message with type "checkpoint-changed" should be sent

  Scenario: Restore deleted file creates file and parent directories
    Given I am in the checkpoint viewer
    And a checkpoint "REFACTOR-003" is selected
    And the file "src/deleted-file.ts" exists in checkpoint but not in working directory
    And the files pane is focused with "src/deleted-file.ts" selected
    When I press the R key
    And I confirm the restoration
    Then the file "src/deleted-file.ts" should be created
    And all parent directories should be created automatically
    And the diff pane should refresh showing the restored file
    And a success message should be displayed

  Scenario: Restore file with uncommitted changes shows warning
    Given I am in the checkpoint viewer
    And a checkpoint "CONFIG-005" is selected
    And the file "src/config.ts" has uncommitted changes in working directory
    And the files pane is focused with "src/config.ts" selected
    When I press the R key
    Then a confirmation dialog should appear with message "Overwrite src/config.ts? Current changes will be LOST."
    And the dialog should have high risk level
    And the dialog should show warning about data loss
    When I press the Y key to confirm
    Then the file "src/config.ts" should be overwritten with checkpoint version
    And the uncommitted changes should be lost
    And the diff pane should refresh showing new comparison
    And a success message should be displayed

  Scenario: Cancel single file restore with N key
    Given I am in the checkpoint viewer
    And a checkpoint "AUTH-001-baseline" is selected
    And the files pane is focused with "src/auth.ts" selected
    When I press the R key
    Then a confirmation dialog should appear
    When I press the N key
    Then the restoration should be cancelled
    And the file "src/auth.ts" should not be modified
    And the viewer should return to normal state

  Scenario: Cancel restore all files with N key
    Given I am in the checkpoint viewer
    And a checkpoint "TUI-001-auto-testing" is selected with 15 files
    When I press the T key
    Then a confirmation dialog should appear
    When I press the N key
    Then the restoration should be cancelled
    And no files should be modified
    And the viewer should return to normal state

  Scenario: R key only works when files pane is focused
    Given I am in the checkpoint viewer
    And a checkpoint "AUTH-001-baseline" is selected
    And the checkpoints pane is focused
    When I press the R key
    Then no restore dialog should appear
    And the viewer should remain unchanged

  Scenario: T key only works when checkpoints or files pane is focused
    Given I am in the checkpoint viewer
    And a checkpoint "TUI-001-auto-testing" is selected
    And the diff pane is focused
    When I press the T key
    Then no restore dialog should appear
    And the viewer should remain unchanged
