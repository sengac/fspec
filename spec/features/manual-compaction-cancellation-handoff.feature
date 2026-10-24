@BUG-214
@integration
@compaction
@context-management
@phase-1
Feature: Manual compaction does not erase cancellation during its handoff
  """
  Architecture notes:
  Manual compaction prepares a working view under the session mutex before
  queueing internal continuation input. Interruption belongs to this operation
  across both stages; internal continuation must not reset an accepted interrupt.
  Exercise the real session-manager compact entry point with a deterministic
  lock barrier and no live provider. Fresh explicit user input remains allowed
  to clear interruption for a new operation. Preserve durable originals and
  release compaction activity when the interrupted operation does not proceed.
  """

  Scenario: Cancellation while manual compaction waits for preparation prevents its continuation
    Given a populated session whose manual compaction is waiting to prepare its working view
    When the session is interrupted before preparation can proceed
    Then manual compaction returns a cancelled outcome without clearing the interrupt
    And no internal continuation input or compaction success is published
    And the original conversation and durable session remain unchanged
    And no compaction activity remains owned by the cancelled operation

  Scenario: Cancellation after manual continuation is queued prevents agent execution
    Given manual compaction has queued its continuation but the agent has not consumed it
    When the session is interrupted before the queued continuation is consumed
    Then the agent discards the cancelled continuation without clearing the interrupt
    And no continuation message is persisted or inference attempted
    And the durable conversation remains unchanged and compaction activity is cleared
