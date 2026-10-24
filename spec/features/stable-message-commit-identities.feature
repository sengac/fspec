@done
@phase-1
@persistence
@session-management
@bug-fix
@BUG-247
Feature: Stable message commit identities
  """
  MessageStore acknowledgement requires file and directory synchronization on
  supported Unix filesystems. The binary index is derived, never the sole copy.
  Cleanup invalidates the old index before replacing JSONL. Fault observation
  records completed real filesystem operations, not source-text assertions.
  Reopening checks consistency, not power-loss survival. See the durability guide
  for supported guarantees and an isolated destructive test procedure.
  """

  Background: User Story
    As an agent recovering a persisted session
    I want authentic history and explicit persistence failures
    So that recovery preserves evidence without inventing outcomes

  Scenario: Stable message retries reconcile ambiguous append acknowledgement
    Given a stable message identity and immutable payload
    When message and index acknowledgement boundaries fail before repeated retries
    Then retries synchronize the existing record without appending a duplicate
    And a cold reader retains its identity payload and original timestamp

  Scenario: Stable message identities reject conflicting payloads
    Given an acknowledged message with a stable identity
    When its role content or metadata conflicts on retry
    Then the conflicting retry fails without altering the archived bytes

  Scenario: Concurrent stable message retries append exactly once
    Given independent writers with the same stable identity and payload
    When both writers acknowledge the message concurrently
    Then rebuilding the derived index retrieves exactly one physical record

  Scenario: Conversation retry reconciles failed manifest acknowledgement
    Given a stable conversation message and committed empty session
    When manifest promotion or acknowledgement fails before repeated retries
    Then the manifest references the original physical message exactly once
