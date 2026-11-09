@done
@persistence
@safety
@bug-fix
@rust
@BUG-597
Feature: The spec write lock is removed when the write finishes
  """
  codelet_common::spec_lock::SpecLock guards every spec/ write with an exclusive OS lock (fs2) on spec/.fspec-write.lock. The file used to persist, so every managed project kept an untracked spec/.fspec-write.lock. Product owner, 2026-11-07: locks are temporary files, cleaned up once used. The last release (reentrant depth reaching zero) removes the file while still holding the lock, then unlocks. Acquire opens or creates the file, locks it, and compares the locked file's identity (same-file Handle: device + inode, or volume + file index on Windows) with the file now at the path; a mismatch or a missing path drops it and starts again, within the lock timeout. A file left by a killed holder is locked by the next writer at once (the OS released it) and removed on release. Limit: an older fspec still running during an upgrade uses the persistent-file protocol, so old and new binaries must not write one project at the same moment then.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The lock file exists while a writer holds the lock and not afterwards: the last release removes it while still holding the lock, then unlocks
  #   2. Acquiring checks that the locked file is still the file at the path, and starts again when it is not, within the lock timeout
  #   3. A lock file left by a killed holder never blocks; the next writer locks it and its release removes it
  #   4. Concurrent writers still serialize: no update is lost and no lock file remains
  #
  # EXAMPLES:
  #   1. After fspec create-story, spec/ holds no .fspec-write.lock
  #   2. A crashed fspec left spec/.fspec-write.lock; the next write takes the lock at once and removes the file
  #   3. Eight processes increment a counter 25 times each under the lock; the counter is 200 and no lock file remains
  #   4. A waiter whose locked file was removed by the releaser starts again on a fresh file
  #
  # ========================================
  Background: User Story
    As a developer whose project fspec manages
    I want to write the board without fspec leaving a runtime lock file in my work tree
    So that git status stays clean and a broad git add never commits fspec's lock

  Scenario: Releasing the lock removes the lock file
    Given a spec directory with no lock file
    When a writer takes the spec lock, takes it again on the same thread, and releases both
    Then spec/.fspec-write.lock exists while the writer holds the lock
    And it still exists after the inner release
    And it is gone after the outer release

  Scenario: A lock file left by a killed holder never blocks
    Given spec/.fspec-write.lock left behind with no process holding it
    When a writer takes the spec lock with a 100 ms timeout
    Then it holds the lock at once
    And the file is gone after it releases

  Scenario: A waiter whose locked file was removed starts again on the file at the path
    Given writer A holds the spec lock
    And writer B is waiting for it
    When A releases the lock
    Then B holds the lock on the file now at spec/.fspec-write.lock
    And no lock file remains after B releases

  Scenario: Concurrent processes serialize and leave no lock file
    Given eight processes that each increment a counter file 25 times under the spec lock
    When they run at the same time
    Then the counter is 200
    And spec/ holds no lock file

  Scenario: An fspec command leaves no lock file
    Given a project fspec manages
    When fspec create-story writes the board
    Then spec/ holds no .fspec-write.lock
