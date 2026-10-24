@done
@phase-1
@persistence
@session-management
@bug-fix
@BUG-247
Feature: Durable message before manifest
  """
  MessageStore acknowledgement requires file and directory synchronization on
  supported Unix filesystems. The binary index is derived, never the sole copy.
  Cleanup invalidates the old index before replacing JSONL. Fault observation
  records completed real filesystem operations, not source-text assertions.
  Reopening checks consistency, not power-loss survival. See the durability guide
  for supported guarantees and an isolated destructive test procedure.
  """

  Scenario: Message barriers precede manifest promotion
    Given a temporary store and a committed empty session
    When a message and its manifest reference are committed
    Then message file and directory barriers precede manifest promotion
    And a cold reader retrieves the referenced content

  Scenario: Failed acknowledgement boundaries preserve recoverable content
    Given a temporary store and a committed empty session
    When each message index and manifest boundary fails in turn
    Then pre-promotion failures leave the old manifest and late failures report promotion
    And all published references remain readable after rebuilding the index

  Scenario: Cleanup replacement preserves referenced content across failures
    Given a temporary store with retained and orphaned records
    When each cleanup durability boundary fails in turn
    Then every retained record survives with correct offsets and promoted failures are explicit

  Scenario: Concurrent durable appends survive a missing derived index
    Given independent writers sharing a temporary store
    When the writers commit messages concurrently and the index is removed
    Then rebuilding retrieves every acknowledged message exactly once

  Scenario: Blob barriers precede message acknowledgement
    Given a temporary blob store and message store
    When blob content and a message referencing it are stored
    Then blob file and directory barriers precede message acknowledgement
    And blob barrier failures prevent acknowledgement and remain retryable
