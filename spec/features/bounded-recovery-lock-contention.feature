@done
@BUG-259
@agent-core
@persistence
@session-management
Feature: Bounded Recovery Lock Contention

  """
  RecoveryLease is shared by persistence spooling and committed session deletion.
  Keep existing lock ordering and inode identity. Explicitly unlock before closing;
  retry only WouldBlock for at most 250 ms with 5 ms backoff. Other I/O errors
  propagate immediately. No global registry mutex is held across acquisition.
  Existing deletion and retained-original regressions remain required.
  """

  Background: User Story
    As a session user
    I want to continue streaming during unrelated shell spawning and short recovery contention
    So that authentic output remains durable without false interruptions

  Scenario: Short recovery contention preserves authentic pending output
    Given a durable session with authentic pending output and a short lived recovery lock holder
    When persistence opens while the holder releases the lock
    Then the original output is recovered exactly once without interruption

  Scenario: Persistent contention is bounded and isolated
    Given a recovery lock held beyond the contention deadline
    When the locked session and an independent session acquire recovery ownership
    Then the independent session progresses and the locked session returns WouldBlock within a finite deadline

  Scenario: Shell spawning does not extend released recovery ownership
    Given a session repeatedly releasing recovery ownership
    When unrelated shells spawn concurrently
    Then every acquisition succeeds without false contention

  Scenario: Non contention filesystem failures propagate immediately
    Given an invalid recovery lock path
    When recovery ownership is acquired
    Then the filesystem error is returned without contention retries
