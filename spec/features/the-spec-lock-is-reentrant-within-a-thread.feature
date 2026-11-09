@done
@infrastructure
@file-ops
@concurrency
@rust
@bug-fix
@critical
@BUG-477
Feature: The spec lock is reentrant within a thread
  """
  Architecture notes:
  - codelet_common::spec_lock::SpecLock keeps a process-wide registry keyed by the canonical directory: the holding thread's nested acquisitions only count up, and the fs2 lock on spec/.fspec-write.lock is released when the outermost guard drops.
  - Other threads and processes open their own handle on the lock file and wait for it, within the LOCK-001 retry budget.
  """

  Background: User Story
    As a developer or agent running several fspec commands at the same time
    I want to have a command that already holds the spec lock take it again without deadlock
    So that commands calling other locking helpers stay serialised against every other writer

  Scenario: The lock is reentrant within a thread and exclusive across processes
    Given a thread that holds the spec lock
    When the same thread acquires the spec lock again
    Then it gets the lock without waiting
    And another process cannot take the spec lock until the outer guard is dropped
