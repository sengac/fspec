@done
@cli
@critical
@workflow-automation
@checkpoint
@git
@phase2
@BUG-027
Feature: Stash system not adding files before creating stash

  """
  Restoration uses manual file operations: git.readBlob() to read checkpoint files, fs.writeFile() to restore. Conflict detection compares byte-by-byte file contents and emits system-reminders before any modifications.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. isomorphic-git HAS native git.stash() API with 'create' operation that creates stash commit without modifying working directory or refs
  #   2. isomorphic-git stash only handles TRACKED files - untracked files must be staged with git.add() BEFORE creating checkpoint for them to be captured
  #   3. Must reset index after checkpoint creation to avoid polluting user's staging area
  #   4. Store checkpoint refs in custom namespace (refs/fspec-checkpoints/{work-unit-id}/{checkpoint-name}) for organization and easy listing
  #   5. Checkpoints must capture both modified tracked files AND new untracked files (respecting .gitignore)
  #   6. Restoration reads checkpoint from custom ref (refs/fspec-checkpoints/{work-unit-id}/{checkpoint-name}) then manually restores files using git.readBlob() and fs.writeFile()
  #   7. Conflict detection compares working directory file contents vs checkpoint file contents byte-by-byte - if different, file is marked as conflicted
  #   8. If conflicts detected, emit system-reminder with conflicted files list and recommended actions (create new checkpoint first) - do NOT modify any files
  #   9. Files in checkpoint but not in working directory are restored (recreated) - files in working directory but not in checkpoint are ignored (left untouched)
  #   10. Cannot use git.stash({ op: 'apply' }) because our custom ref namespace (refs/fspec-checkpoints) is incompatible with isomorphic-git's reflog-based stash access
  #   11. Detect and handle conflicts. Compare file contents byte-by-byte. If conflicts detected, emit system-reminder with conflicted files and recommend creating new checkpoint first. Do NOT overwrite without user awareness.
  #
  # EXAMPLES:
  #   1. Current bug: git.commit() called without git.add(), creates empty commit that captures nothing
  #   2. User modifies tracked file README.md, runs checkpoint, file is staged then stashed with git.stash({ op: 'create' }), working directory unchanged
  #   3. User creates new untracked file auth.test.ts, runs checkpoint, file is staged with git.add() then captured by stash, working directory unchanged
  #   4. User modifies README.md and creates new.ts, runs checkpoint, both files staged and captured, index reset afterward, working directory unchanged
  #   5. User restores checkpoint, files from stash commit are read and written back to working directory, all files (tracked + previously-staged untracked) restored
  #   6. User creates checkpoint 'baseline', modifies files more, runs restore, all files read from checkpoint commit using git.readBlob() and written to working directory
  #   7. User creates checkpoint with file A (v1), modifies file A to v2, runs restore, conflict detected (contents differ), system-reminder emitted, file A NOT overwritten (stays v2)
  #   8. User creates checkpoint with file A, deletes file A from working directory, runs restore, file A is recreated from checkpoint (no conflict)
  #   9. User creates checkpoint, adds new file B (not in checkpoint), runs restore, file B is left untouched (ignored), only checkpoint files restored
  #   10. User tries to restore non-existent checkpoint name, git.resolveRef() throws error, restoration returns success=false with 'checkpoint not found' message
  #
  # QUESTIONS (ANSWERED):
  #   Q: What is the stash system currently doing wrong? When I create a checkpoint, what files should be saved and what actually gets saved?
  #   A: true
  #
  #   Q: Should the checkpoint system stash both tracked modified files AND untracked new files? Or only tracked files?
  #   A: true
  #
  #   Q: Should we update the feature specification (intelligent-checkpoint-system-for-workflow-transitions.feature line 11) to document that files must be staged before stashing, not using native git -u flag?
  #   A: true
  #
  #   Q: For checkpoint restoration, should we attempt to detect and handle merge conflicts (if user modified files since checkpoint), or simply overwrite with checkpoint contents?
  #   A: true
  #
  #   Q: Should checkpoint restoration preserve file permissions and timestamps, or just content?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Yes, update the feature spec to document that isomorphic-git requires staging untracked files with git.add() before git.stash({ op: 'create' }) can capture them. This is a limitation of isomorphic-git, not native git.
  #   2. Just content. File permissions and timestamps are not critical for checkpoint use case (experimentation and rollback). Focus on content restoration only.
  #
  # ========================================

  Background: User Story
    As a developer using fspec checkpoints
    I want to create and restore checkpoints that actually capture all file changes
    So that I can safely experiment and rollback without losing work

  Scenario: Current bug - empty commit captures nothing
    Given I have a work unit "BUG-027" in "implementing" status
    And I have modified tracked file "README.md"
    When the current broken implementation calls git.commit() without git.add()
    Then an empty commit is created
    And no files are captured in the checkpoint
    And the checkpoint is useless for restoration

  Scenario: Create checkpoint with modified tracked file
    Given I have a work unit "BUG-027" in "implementing" status
    And I have modified tracked file "README.md" with new content
    When I run "fspec checkpoint BUG-027 baseline"
    Then the file should be staged with git.add()
    And a checkpoint commit should be created with git.stash({ op: 'create' })
    And the working directory should remain unchanged
    And the index should be reset after checkpoint creation

  Scenario: Create checkpoint with new untracked file
    Given I have a work unit "BUG-027" in "implementing" status
    And I have created new untracked file "auth.test.ts"
    When I run "fspec checkpoint BUG-027 baseline"
    Then the file should be staged with git.add()
    And the file should be captured by git.stash({ op: 'create' })
    And the working directory should remain unchanged
    And the index should be reset after checkpoint creation

  Scenario: Create checkpoint with both tracked and untracked files
    Given I have a work unit "BUG-027" in "implementing" status
    And I have modified tracked file "README.md"
    And I have created new untracked file "new.ts"
    When I run "fspec checkpoint BUG-027 baseline"
    Then both files should be staged with git.add()
    And both files should be captured in the checkpoint
    And the checkpoint ref should be stored at "refs/fspec-checkpoints/BUG-027/baseline"
    And the index should be reset afterward
    And the working directory should remain unchanged

  Scenario: Restore checkpoint successfully
    Given I have created checkpoint "baseline" for work unit "BUG-027" with files A and B
    And I have modified files A and B further
    When I run "fspec restore-checkpoint BUG-027 baseline"
    Then files should be read from checkpoint commit using git.readBlob()
    And files should be written back to working directory using fs.writeFile()
    And all files from checkpoint should be restored
    And the restoration should succeed

  Scenario: Restore checkpoint with clean working directory
    Given I have created checkpoint "baseline" for work unit "BUG-027"
    And I have modified files further
    And I have committed all changes
    When I run "fspec restore-checkpoint BUG-027 baseline"
    Then all checkpoint files should be read using git.readBlob()
    And files should be written to working directory
    And the restoration should complete successfully

  Scenario: Detect conflict when file modified since checkpoint
    Given I have created checkpoint "baseline" with file "config.ts" version 1
    And I have modified "config.ts" to version 2 in working directory
    When I run "fspec restore-checkpoint BUG-027 baseline"
    Then conflict detection should compare file contents byte-by-byte
    And "config.ts" should be marked as conflicted
    And a system-reminder should be emitted with conflicted file paths
    And the system-reminder should recommend creating new checkpoint first
    And "config.ts" should NOT be overwritten
    And file should remain as version 2
    And restoration should return success=false

  Scenario: Restore deleted file from checkpoint
    Given I have created checkpoint "baseline" with file "utils.ts"
    And I have deleted "utils.ts" from working directory
    When I run "fspec restore-checkpoint BUG-027 baseline"
    Then "utils.ts" should be recreated from checkpoint
    And no conflict should be detected
    And the restoration should succeed

  Scenario: Restore ignores files not in checkpoint
    Given I have created checkpoint "baseline" with files A and B
    And I have added new file "extra.ts" to working directory
    When I run "fspec restore-checkpoint BUG-027 baseline"
    Then files A and B should be restored from checkpoint
    And file "extra.ts" should be left untouched
    And only checkpoint files should be restored

  Scenario: Error when restoring non-existent checkpoint
    Given I have a work unit "BUG-027"
    And checkpoint "nonexistent" does not exist
    When I run "fspec restore-checkpoint BUG-027 nonexistent"
    Then git.resolveRef() should throw an error
    And restoration should return success=false
    And error message should be "Checkpoint 'nonexistent' not found for work unit BUG-027"
    And no files should be modified
