@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Refuse recovery into a deleted session transcript
  """
  Retained transcript records use BUG-246's shared owner and core RecoveryLease.
  Explicit retry retires ownership after confirmed deletion instead of promoting
  its bytes. Missing session acknowledgement is not a new transcript commit.
  Unreadable manifests remain errors and never authorize retirement.
  """

  Scenario: Recovery cannot recreate a deleted session transcript
    Given a deleted session with a retained unacknowledged transcript record
    When explicit persistence recovery is attempted after transcript removal
    Then recovery retires pending ownership without recreating the deleted transcript
