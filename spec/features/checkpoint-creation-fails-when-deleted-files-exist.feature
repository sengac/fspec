@done
@git-integration
@checkpoint-management
@high
@git
@checkpoint
@bug-fix
@GIT-011
Feature: Checkpoint creation fails when deleted files exist
  """
  Uses isomorphic-git statusMatrix to detect file changes. git.statusMatrix returns [filepath, HEAD, WORKDIR, STAGE] tuples. WORKDIR=0 means file deleted. Must use git.remove() for deleted files (WORKDIR=0) and git.add() for modified/added files (WORKDIR=2). All changes captured in single git.stash({ op: 'create' }) commit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint creation must detect file status using git.statusMatrix to determine if files are modified, added, or deleted
  #   2. Deleted files must be staged using git.remove() instead of git.add()
  #   3. Modified and added files must continue using git.add() for staging
  #   4. Checkpoint must capture all changed files (modified, added, and deleted) in a single stash commit
  #
  # EXAMPLES:
  #   1. Developer deletes README.md and runs 'fspec checkpoint DOC-001 baseline' - checkpoint is created successfully with deleted file tracked
  #   2. Developer has mixed changes (modified src/app.ts, deleted docs/old.md, added tests/new.test.ts) - checkpoint captures all three types of changes
  #   3. Developer deletes multiple files in docs/ directory - all deletions are staged using git.remove() and captured in checkpoint
  #   4. Restoring checkpoint that includes deleted files recreates those files in working directory
  #
  # ========================================
  Background: User Story
    As a developer using fspec checkpoints
    I want to create checkpoints when deleted files exist
    So that I can safely experiment and rollback changes including file deletions

  Scenario: Create checkpoint with deleted file
    Given I have a git repository with a committed file "README.md"
    When I delete "README.md" from the working directory
    And I run "fspec checkpoint DOC-001 baseline"
    Then the checkpoint should be created successfully
    And the checkpoint should track the deleted file "README.md"
    And the output should show "Captured 1 file(s)"

  Scenario: Create checkpoint with mixed changes
    Given I have a git repository with committed files
    When I modify "src/app.ts"
    And I delete "docs/old.md"
    And I add a new file "tests/new.test.ts"
    And I run "fspec checkpoint WORK-001 mixed-changes"
    Then the checkpoint should be created successfully
    And the checkpoint should capture all 3 changed files
    And the output should show "Captured 3 file(s)"

  Scenario: Create checkpoint with multiple deleted files
    Given I have a git repository with multiple files in "docs/" directory
    When I delete all files in "docs/" directory
    And I run "fspec checkpoint DOC-002 remove-docs"
    Then the checkpoint should be created successfully
    And all deleted files should be staged using git.remove()
    And the checkpoint should capture all deleted files

  Scenario: Restore checkpoint that includes deleted files
    Given I have created a checkpoint with deleted files
    When I restore the checkpoint
    Then the deleted files should be recreated in the working directory
    And the working directory should match the checkpoint state
