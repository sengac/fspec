@BUG-582
@done
@agent-core
@session-management
@persistence
@BUG-250
@BUG-571
Feature: Retire pending transcript recovery with authoritative session deletion
  """
  Recovery and deletion acquire the stable recovery lease before session
  ownership. Only an absent committed manifest retires authentic pending data.
  A promoted deletion remains distinguishable from a retryable precommit failure.
  BUG-571: in the ownership race, recovery must own the recovery lease before
  deletion starts (deletion then fails fast on lease contention); the test
  observes that ownership with a non-blocking probe of the lease file instead
  of the 200 ms head start it once relied on.
  """

  Scenario: Canonical deletion retires durable pending transcript recovery
    Given a real session with a cold durable pending transcript record
    When canonical deletion settles before cold recovery runs
    Then the pending lane is removed and recovery cannot recreate its transcript

  Scenario: Stale recovery bytes cannot resurrect a deleted transcript
    Given a retained copy of a durable transcript recovery record
    When those bytes are recovered after the owning session is deleted
    Then neither a transcript nor a conversation original is recreated

  Scenario: Live transcript recovery remains authentic and idempotent
    Given a live session with a cold durable pending transcript record
    When cold recovery retries that record twice
    Then exactly one authentic transcript record survives without a conversation original

  Scenario: Deleted sessions retire caller-owned recovery memory
    Given a transcript original retained in memory after recovery publication fails
    When the session is deleted and the retained admission is retried
    Then its recovery owner is empty and neither private lane is recreated

  Scenario: Recovery and deletion respect active session ownership
    Given pending transcript recovery and an active session writer transaction
    When recovery and deletion race with that transaction
    And recovery owns the recovery lease before deletion starts
    Then recovery cannot acknowledge before ownership and deletion returns without an inverted lock wait

  Scenario: Failed deletion before promotion preserves recovery originals
    Given durable pending transcript recovery and an unrelated live session
    When manifest deletion fails before promotion
    Then both sessions and the exact pending bytes remain unchanged for retry

  Scenario: Failed cleanup after deletion reports promotion and remains retryable
    Given a session with durable pending private transcript data
    When recovery retirement fails after manifest deletion
    Then deletion reports promotion and retry removes residual data without transcript resurrection
