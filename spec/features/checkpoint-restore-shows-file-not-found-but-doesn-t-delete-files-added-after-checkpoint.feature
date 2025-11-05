@done
@checkpoint-management
@git
@high
@GIT-012
Feature: Checkpoint restore shows 'File not found' but doesn't delete files added after checkpoint
  """
  Uses git.walk() to compare checkpoint tree against HEAD tree. Uses git.readBlob() to read checkpoint files. Restoration deletes files that exist in HEAD but not in checkpoint (manual deletion with fs.unlink). Diff viewer uses getCheckpointFileDiff() to show restore preview. Only tracked files are considered for deletion (gitignored files unaffected).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint restore must fully restore working directory to exact state at checkpoint creation time
  #   2. Files that exist in HEAD but not in checkpoint must be deleted during restore
  #   3. Diff viewer must clearly indicate when files will be deleted on restore (not just 'File not found in checkpoint')
  #   4. File deletion happens automatically as part of normal restore operation. No force flag required since restoring to checkpoint state inherently means removing files added after checkpoint.
  #   5. Only tracked files should be considered for deletion. Gitignored files and node_modules are not affected by checkpoint restore.
  #
  # EXAMPLES:
  #   1. Checkpoint created at T1 contains [A.txt, B.txt], then C.txt is added at T2. Restoring checkpoint should delete C.txt and restore A.txt, B.txt to T1 state.
  #   2. Diff viewer shows file D.txt with message 'Will be deleted on restore' instead of 'File not found in checkpoint' when D.txt exists in HEAD but not in checkpoint
  #   3. Restoring checkpoint deletes 3 files [new-feature.ts, test.spec.ts, README-draft.md] that were added after checkpoint, and restores 2 modified files [main.ts, config.json] to checkpoint state
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should deleted files be backed up somewhere (e.g., temporary checkpoint) before deletion, or rely on user to create manual checkpoint first?
  #   A: true
  #
  #   Q: Should file deletion count as a 'conflict' that requires --force flag, or should it happen automatically since it's part of restoring to checkpoint state?
  #   A: true
  #
  #   Q: What about files that are gitignored or in node_modules? Should those be considered for deletion, or only tracked files?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No backup needed - files are in HEAD so user can restore with git if needed. Existing warning to create manual checkpoint first is sufficient.
  #
  # ========================================
  Background: User Story
    As a developer using fspec checkpoints
    I want to restore a checkpoint to recover previous work state
    So that I get an exact restoration of the working directory as it was when the checkpoint was created

  Scenario: Restore checkpoint deletes files added after checkpoint creation
    Given a checkpoint "baseline" was created containing files "A.txt" and "B.txt"
    And a new file "C.txt" was added after the checkpoint
    When I restore checkpoint "baseline"
    Then file "C.txt" should be deleted from the working directory
    And files "A.txt" and "B.txt" should be restored to their checkpoint state
    And the working directory should match the exact state at checkpoint creation

  Scenario: Diff viewer shows clear deletion message for files not in checkpoint
    Given a checkpoint "baseline" exists
    And file "D.txt" exists in HEAD but not in the checkpoint
    When I view the checkpoint diff for "D.txt"
    Then the diff should show "Will be deleted on restore" instead of "File not found in checkpoint"
    And the message should clearly indicate the file will be removed during restoration

  Scenario: Restore checkpoint deletes multiple new files and restores modified files
    Given a checkpoint "before-changes" was created
    And 3 new files were added after checkpoint: "new-feature.ts", "test.spec.ts", "README-draft.md"
    And 2 files were modified after checkpoint: "main.ts", "config.json"
    When I restore checkpoint "before-changes"
    Then the 3 new files should be deleted: "new-feature.ts", "test.spec.ts", "README-draft.md"
    And the 2 modified files should be restored to checkpoint state: "main.ts", "config.json"
    And no files added after checkpoint should remain in the working directory
