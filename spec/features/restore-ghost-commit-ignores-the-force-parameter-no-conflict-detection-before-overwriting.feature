@git
@checkpoint-management
@bug-fix
@WT-010
Feature: restore_ghost_commit ignores the force parameter — no conflict detection before overwriting
  """
  restore_ghost_commit (rust/git/src/ghost_commit.rs) takes force: bool but never reads it (underscore-prefixed). Fix: after resolving the checkpoint tree and collecting current workdir files (the same two maps already computed for the restore loop), when force == false compute the divergent file set (checkpoint-vs-workdir: modified, deleted-since-checkpoint, added-after-checkpoint — identical logic to get_checkpoint_diff_files) and, if non-empty, return Err(GitError::RestoreConflict { files }) BEFORE any fs::write/fs::remove_file. force == true keeps the existing unconditional clobber. A new GitError::RestoreConflict variant is added (the existing ConflictError Display is session-merge specific: "modified in both session and main worktree"). fspec-core restore-checkpoint (restore_checkpoint.rs::restore_util) must map RestoreConflict to a conflict result (success:false, conflicts_detected:true, conflicted_files) instead of falling into the not-found sentinel (Err(_) => not_found_result).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. restore_ghost_commit must honor its force parameter: when force is false and the working tree differs from the checkpoint (modified, deleted, or added files), it must return Err(GitError::RestoreConflict { files }) WITHOUT writing or deleting any file; when force is true it keeps today's unconditional clobber semantics
  #   2. Conflict detection for force=false must be checkpoint-tree vs working-dir based (the same machinery get_checkpoint_diff_files uses: checkpoint tree files vs collect_worktree_files), NOT based on git status/index dirtiness — so a restore of a checkpoint whose state matches the working dir proceeds cleanly even in a dirty repo
  #   3. fspec-core restore-checkpoint must surface a codelet-git ConflictError as a conflict result (conflictsDetected=true + the conflicted files), not as the 'checkpoint not found' sentinel — so the force=false conflict path reaches the user with the right message
  #
  # EXAMPLES:
  #   1. Checkpoint taken with test.txt='v1'; user edits test.txt to 'v2' afterwards; restore with force=false → Err(ConflictError { files: ["test.txt"] }) and test.txt on disk is still 'v2'
  #   2. Same setup (test.txt modified after checkpoint), but restore with force=true → Ok(RestoreResult) and test.txt on disk is back to 'v1' (today's clobber semantics, unchanged)
  #   3. A file added after the checkpoint exists in the working dir; restore with force=false → Err(ConflictError) listing the added file, and the added file is NOT deleted
  #
  # ========================================
  Background: User Story
    As a developer using fspec checkpoints
    I want to restore a checkpoint without force and get a conflict error instead of silently losing local modifications
    So that checkpoint restore can no longer destroy uncommitted work made after the checkpoint was taken

  Scenario: Non-forced restore reports a conflict instead of overwriting modified files
    Given a git repository with a ghost commit checkpoint whose file test.txt contains 'v1' and test.txt has been edited to 'v2' after the checkpoint
    When I restore the checkpoint with force set to false
    Then the restore returns a conflict error listing test.txt and test.txt on disk still contains 'v2'

  Scenario: Forced restore overwrites modified files without conflict detection
    Given a git repository with a ghost commit checkpoint whose file test.txt contains 'v1' and test.txt has been edited to 'v2' after the checkpoint
    When I restore the checkpoint with force set to true
    Then the restore succeeds and test.txt on disk contains 'v1' again

  Scenario: Non-forced restore refuses to delete files added after the checkpoint
    Given a git repository with a ghost commit checkpoint and a file added-after-checkpoint.txt that exists only in the working directory
    When I restore the checkpoint with force set to false
    Then the restore returns a conflict error listing added-after-checkpoint.txt and the file still exists on disk
