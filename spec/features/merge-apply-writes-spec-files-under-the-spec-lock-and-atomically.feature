@done
@git
@session
@session-merge
@bug-fix
@rust
@WT-014
Feature: Applying a worktree merge writes spec files without the spec lock or an atomic replace
  """
  apply_session_changes (rust/git/src/session_result.rs) takes codelet_common::spec_lock::SpecLock on main's spec/ directory before it snapshots main's working tree for conflict detection and holds it until the apply is done, so no fspec or TUI board write can land between the conflict check and the copy, and none is overwritten. Every blob the merge writes into main goes through codelet_common::spec_lock::replace_file_atomically (write_file_with_mode in rust/git/src/tree_snapshot.rs), the same temp-file, fsync and rename every spec write uses. A lock timeout surfaces as the standard 'Failed to acquire file lock on ... Check for stuck fspec processes.' error before anything is copied. A main checkout without spec/ whose session adds nothing under spec/ takes no lock. Until WT-015 adds the state-root resolver, the lock directory is the main checkout's spec/. WT-017 (product owner decision 2026-11-07, WT-015: the board lives in the one shared state root and only spec/features travel with the branch): the merge never writes shared spec state such as spec/work-units.json, so these scenarios merge a branch spec file, spec/features/merge.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Applying a merge holds main's spec write lock (spec/.fspec-write.lock) from the moment it reads main's files for conflict detection until the last file is written; WT-015 later moves this lock to the shared state root
  #   2. Every file the merge writes into main is replaced atomically (temp file in the same directory, fsync, rename), so a reader sees the old or the new file and never a partial one
  #   3. When another writer holds the lock past the lock timeout, the merge fails with the standard lock message before anything is copied, and the session worktree stays intact for a retry
  #   4. The merge releases the lock when the apply ends, whether it succeeded or failed
  #   5. A main checkout with no spec/ directory, whose session also adds nothing under spec/, has no spec state to protect and takes no lock
  #
  # EXAMPLES:
  #   1. Another process holds main's spec lock and FSPEC_SPEC_LOCK_TIMEOUT_MS is 0; merging a session that changed spec/features/merge.feature fails with 'Failed to acquire file lock on <main>/spec/.fspec-write.lock', main's board is unchanged and the worktree still exists
  #   2. A session changed spec/features/merge.feature and hello.txt; after the merge both files in main are new inodes holding the session's content, and no temp files are left beside them
  #   3. After a merge finishes, another writer acquires main's spec lock at once
  #   4. A merge stops on a conflict in spec/features/merge.feature; another writer then acquires main's spec lock at once
  #
  # ========================================
  Background: User Story
    As a developer merging an isolated session while fspec or the TUI writes the board
    I want to have the merge write main's spec files under the spec write lock and by atomic replace
    So that a concurrent board write is never lost and no reader ever sees a half-written spec file

  Scenario: A spec lock held past the timeout fails the merge before anything is applied
    Given a temp git repo with spec/features/merge.feature committed, and an isolated session that changed spec/features/merge.feature
    And another writer holds main's spec lock and the lock timeout is 0 ms
    When the session is merged
    Then the merge fails with the standard lock message naming main's spec/.fspec-write.lock
    And main's spec/features/merge.feature is unchanged and nothing is committed
    And the session worktree still exists

  Scenario: Merged files replace main's files atomically and the lock is released
    Given a temp git repo with spec/features/merge.feature and hello.txt committed, and an isolated session that changed both
    When the session is merged
    Then main's spec/features/merge.feature and hello.txt are new files holding the session's content
    And no temporary files are left beside them
    And another writer acquires main's spec lock at once

  Scenario: A merge that stops on a conflict releases the lock
    Given a temp git repo with spec/features/merge.feature committed, and an isolated session that changed its first line
    And main changed the same line in its working tree
    When the session is merged
    Then the merge reports a conflict in spec/features/merge.feature
    And another writer acquires main's spec lock at once
