@done
@GIT-017
@git
@rust
@checkpoint-management
@wip
Feature: Ghost commits for checkpoint snapshots
  """
  Implement ghost_commit module in rust/git crate with create_ghost_commit() and restore_ghost_commit() functions
  Use gix temporary index (GIT_INDEX_FILE equivalent) via gix::worktree::state API to avoid disturbing staging area
  Ghost commit creation: gix::Object::write_tree() + gix::commit::create() with no ref update
  Keep existing refs/fspec-checkpoints/* ref storage but store ghost commit SHA instead of stash OID
  Expose NAPI bindings: createGhostCheckpoint() and restoreGhostCheckpoint() for TypeScript consumption
  Update src/utils/git-checkpoint.ts to call Rust NAPI bindings instead of isomorphic-git
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MUST use pure gitoxide (gix) Rust implementation - NO git CLI commands allowed anywhere
  #   2. Ghost commits must capture complete working tree state including staged, unstaged, and untracked files
  #   3. Ghost commits have no branch reference - invisible to git log without explicit SHA
  #   4. Ghost commits preserve parent relationship to HEAD at creation time
  #   5. Temporary index must be used to avoid disturbing user's staging area
  #   6. Must maintain same TypeScript API surface as current stash-based implementation
  #   7. Checkpoint metadata must store ghost commit SHA instead of stash ref
  #
  # EXAMPLES:
  #   1. Developer creates checkpoint with staged, unstaged, and untracked files - all captured in ghost commit
  #   2. Developer creates checkpoint while files are staged - staging area remains undisturbed after checkpoint
  #   3. Developer restores checkpoint - working tree files replaced with checkpoint contents
  #   4. Multiple checkpoints for same work unit - each has unique ghost commit SHA stored in refs
  #   5. Git log shows no trace of ghost commits - only accessible via explicit SHA
  #   6. Restore checkpoint when files were added after checkpoint - new files deleted to match exact state
  #
  # ========================================
  Background: User Story
    As a developer
    I want to use ghost commits for checkpoint snapshots
    So that checkpoints are faster, more reliable, and use pure Rust git operations

  @checkpoint
  @crud
  Scenario: Create checkpoint capturing all file states
    Given I have a git repository with uncommitted changes
    And I have staged files in the index
    And I have unstaged modifications to tracked files
    And I have untracked files in the working directory
    When I create a ghost commit checkpoint named "test-checkpoint"
    Then all file states should be captured in the ghost commit
    And the checkpoint should store a valid git commit SHA
    And the checkpoint should be stored under refs/fspec-checkpoints/

  @checkpoint
  @file-ops
  Scenario: Checkpoint creation preserves staging area
    Given I have a git repository with a file staged for commit
    And I have additional unstaged changes
    When I create a ghost commit checkpoint
    Then the staging area should remain unchanged
    And the same files should still be staged
    And the checkpoint should capture all working tree state

  @checkpoint
  @restore
  Scenario: Restore checkpoint replaces working tree files
    Given I have a git repository with a ghost commit checkpoint
    And the checkpoint contains specific file contents
    And I have modified files since the checkpoint was created
    When I restore the checkpoint
    Then the working tree files should match the checkpoint contents
    And modified files should be reverted to checkpoint state

  @checkpoint
  @multiple
  Scenario: Multiple checkpoints have unique SHA identifiers
    Given I have a git repository with uncommitted changes
    When I create a ghost commit checkpoint named "checkpoint-1"
    And I modify some files
    And I create a ghost commit checkpoint named "checkpoint-2"
    Then each checkpoint should have a unique SHA
    And both checkpoints should be independently restorable
    And the refs should be stored under refs/fspec-checkpoints/<work-unit-id>/

  @checkpoint
  @git-ops
  Scenario: Ghost commits are invisible to git log
    Given I have a git repository with a ghost commit checkpoint
    When I run git log to view repository history
    Then the ghost commit should not appear in the log
    And the ghost commit should only be accessible via explicit SHA reference

  @checkpoint
  @restore
  @cleanup
  Scenario: Restore checkpoint deletes files added after checkpoint
    Given I have a git repository with a ghost commit checkpoint
    And I create new files after the checkpoint was created
    When I restore the checkpoint
    Then the new files should be deleted
    And the working tree should match the exact state at checkpoint creation

  @checkpoint
  @git-ops
  Scenario: Ghost commit preserves parent relationship to HEAD
    Given I have a git repository with committed history
    And I note the current HEAD commit SHA
    And I have uncommitted changes in the working directory
    When I create a ghost commit checkpoint
    Then the ghost commit's parent should be the noted HEAD SHA
    And the ghost commit should be a valid commit object with proper tree reference
