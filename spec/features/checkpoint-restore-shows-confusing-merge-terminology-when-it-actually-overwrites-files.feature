@workflow-automation
@done
@cli
@checkpoint
@git-ops
@BUG-053
Feature: Checkpoint restore shows confusing 'merge' terminology when it actually overwrites files
  """
  The bug is in restore-checkpoint.ts lines 77-80 where option 3 is labeled 'Force restore with merge' and describes 'Attempts to merge changes' which is completely false - the implementation never calls git.merge() or creates conflict markers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Current behavior: restore-checkpoint shows 3 options when working directory is dirty: (1) Commit changes first, (2) Stash changes and restore, (3) Force restore with merge
  #   2. Option 3 'Force restore with merge' is misleading - it doesn't actually merge, it just overwrites files when force=true is passed
  #   3. The restoreCheckpointUtil in git-checkpoint.ts with force=true overwrites files directly without any merge logic (lines 329-366)
  #
  # EXAMPLES:
  #   1. User has uncommitted changes, runs 'fspec restore-checkpoint AUTH-001 checkpoint-name', sees confusing 'Force restore with merge' option, expects simpler 'Overwrite files' language
  #
  # QUESTIONS (ANSWERED):
  #   Q: What should option 3 be renamed to? Suggestions: 'Force overwrite files', 'Discard changes and restore', 'Overwrite with checkpoint', or something else?
  #   A: true
  #
  #   Q: Should the risk level stay 'High' for the overwrite option? Or change to something else?
  #   A: true
  #
  #   Q: Should the description explain data loss risk? E.g., 'Overwrites working directory with checkpoint. Current changes will be lost forever unless committed or stashed.'
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to restore a checkpoint and overwrite my files
    So that I don't get confused by merge terminology when I just want to replace files

  Scenario: Restore with dirty working directory shows accurate overwrite option
    Given I have a work unit AUTH-001 with a checkpoint named 'baseline'
    And I have uncommitted changes in my working directory
    When I run 'fspec restore-checkpoint AUTH-001 baseline'
    Then option 3 should be labeled 'Overwrite files (discard changes)'
    And option 3 should have risk level 'High'
    And option 3 description should warn 'Overwrites working directory with checkpoint. Current changes will be LOST FOREVER unless committed or stashed.'

  Scenario: Option text removes all misleading merge terminology
    Given I have a work unit with uncommitted changes
    When I view the restore-checkpoint prompt options
    Then no option should contain the word 'merge'
    And no option description should mention 'conflicts' or 'manual resolution'
    And the terminology should accurately reflect pure file overwrite behavior
