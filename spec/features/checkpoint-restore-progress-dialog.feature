@done
@checkpoint-management
@high
@tui
@checkpoint
@dialog
@progress
@TUI-027
Feature: Checkpoint Restore Progress Dialog
  """
  Error handling: Show error state in same dialog with red color, manual dismissal required. No cancellation during restore operation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Dialog must appear when user confirms restore (single file or all files)
  #   2. Dialog must show the current file being restored in real-time
  #   3. Dialog must show count of remaining files to restore
  #   4. Dialog must auto-close 3 seconds after all files are restored
  #   5. StatusDialog component must be reusable for other progress operations
  #   6. Display only the current file being processed (not a scrollable list)
  #   7. Yes, show progress indicator with format 'X/Y files' or percentage
  #   8. Show error state in dialog with manual dismissal required
  #   9. No cancellation allowed once restore starts
  #   10. Allow early dismissal with ESC during 3-second completion notice
  #   11. Dialog appears for both single and multi-file restores
  #
  # EXAMPLES:
  #   1. User restores 5 files: dialog shows 'Restoring src/foo.ts (1/5)', then 'Restoring src/bar.ts (2/5)', etc.
  #   2. User restores single file: dialog shows 'Restoring src/test.ts (1/1)', then shows completion notice for 3 seconds
  #   3. All files restored: dialog changes to 'Restore Complete\! Closing in 3 seconds...' with countdown, user can press ESC to close early
  #   4. Error during restore: dialog shows 'Error: Failed to restore src/config.ts' with red styling, user must press ESC to dismiss
  #   5. StatusDialog component can be reused: accepts props like currentItem, totalItems, status, errorMessage
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the dialog show a scrollable list of ALL files being restored, or just display the current file name being processed?
  #   A: true
  #
  #   Q: Should there be a progress bar or percentage indicator (e.g., '3/10 files' or '30%')?
  #   A: true
  #
  #   Q: What should happen if an error occurs during restore? Should the dialog show an error state and require manual dismissal?
  #   A: true
  #
  #   Q: Should users be able to cancel the restore operation in progress, or is it non-cancellable once started?
  #   A: true
  #
  #   Q: During the 3-second completion notice, should the user be able to dismiss it early with ESC, or must they wait the full 3 seconds?
  #   A: true
  #
  #   Q: For single file restore, should the dialog still appear (showing 1/1 file), or only for multi-file restores?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using checkpoint restore
    I want to see real-time progress during file restoration
    So that I understand what's happening and when the restore is complete

  Scenario: Multi-file restore with progress tracking
    Given I have a checkpoint with 5 files to restore
    When I confirm restore all files
    Then a StatusDialog should appear showing 'Restoring src/foo.ts (1/5)'
    Then the dialog should update to 'Restoring src/bar.ts (2/5)'
    Then this should continue for all 5 files

  Scenario: Single file restore with auto-close
    Given I have a checkpoint with 1 file to restore
    When I confirm restore of the single file
    Then StatusDialog should appear showing 'Restoring src/test.ts (1/1)'
    Then the dialog should change to completion notice after restore
    Then the dialog should auto-close after 3 seconds

  Scenario: Completion notice with early dismissal
    Given all files have been restored successfully
    When StatusDialog shows 'Restore Complete\! Closing in 3 seconds...'
    Then I should see a countdown timer
    When I press ESC before 3 seconds elapse
    Then the dialog should close immediately

  Scenario: Error during restore with manual dismissal
    Given I am restoring files from a checkpoint
    When an error occurs while restoring 'src/config.ts'
    Then StatusDialog should show 'Error: Failed to restore src/config.ts'
    Then the error message should be displayed with red styling
    Then I must press ESC to dismiss the dialog

  Scenario: StatusDialog reusability for other operations
    Given StatusDialog is a reusable component
    When I use StatusDialog for a different operation
    Then it should accept props: currentItem, totalItems, status, errorMessage
    Then it should display progress for any batch operation
    Then it should handle completion notice with auto-close behavior
