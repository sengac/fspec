@BUG-214
@phase-1
@cli
@compaction
@context-management
Feature: Cancellation owns automatic compaction until durable promotion
  """
  Architecture notes:
  The direct pre-prompt and stream-recovery builders must observe the same
  interruption state and wakeup as ordinary inference. A deadline is not user
  cancellation. Awaiting exploration, correction or synthesis must be cancellable
  without a provider response. Recheck cancellation after selected serialization
  within admission, before promoting a replacement. Once promotion has won,
  preserve and reconcile that authoritative generation but never resume the task
  after a user interrupt. Cancellation remains non-retryable and clears activity.
  Existing terminal-recovery, explicit-retry and cross-target guards remain valid.
  """

  Scenario: Cancellation stops a pending pre-prompt compaction
    Given an unfinished task triggers pre-prompt compaction with a pending builder
    When the user interrupts the session before the builder responds
    Then compaction stops promptly with an explicit cancelled outcome
    And the durable conversation and checkpoint remain unchanged
    And no compaction success or continuation request is emitted

  Scenario: Cancellation stops a pending hook-recovery compaction
    Given an unfinished task triggers hook recovery with a pending builder
    When the user interrupts the session before the builder responds
    Then compaction stops promptly with an explicit cancelled outcome
    And the durable conversation and checkpoint remain unchanged
    And no compaction success or continuation request is emitted

  Scenario: A late candidate cannot override cancellation
    Given an automatic compaction builder is about to return a valid candidate
    When the user interrupts before the candidate is admitted
    Then the candidate is rejected as cancelled without durable promotion
    And no compaction success or continuation request is emitted

  Scenario: Cancellation during admission vetoes promotion
    Given a valid compaction candidate reaches selected request admission
    When the user interrupts while its replacement request is serialized
    Then the candidate is rejected as cancelled without durable promotion
    And the previous context tokens and provider checkpoint remain unchanged

  Scenario: Cancellation after promotion preserves the authoritative generation
    Given automatic compaction has durably promoted a valid generation
    When the user interrupts during committed context publication
    Then the promoted generation remains authoritative and reconciled
    And the original task does not issue a continuation request

  Scenario: An explicit builder retry vetoes cancellation during admission
    Given fresh user input restarts a previously failed compaction builder
    When the user interrupts during replacement request admission
    Then the retry returns cancellation without promoting a generation
    And another fresh input cannot retry the cancelled operation
