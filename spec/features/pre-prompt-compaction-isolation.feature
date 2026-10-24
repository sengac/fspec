@cli
@context-management
@compaction
@CMPCT-079
Feature: Pre-prompt compaction isolation
  """
  Threshold-triggered compaction uses the selected durable DAG builder before
  ordinary inference. Failure or cancellation preserves authoritative history
  and counters, blocks inference and never publishes completion.
  """

  Scenario: Missing pre-prompt builder preserves authoritative state
    Given a persisted conversation above its selected compaction threshold
    When pre-prompt compaction has no registered builder
    Then authoritative messages turns counters and durable envelopes remain unchanged
    And no provider request or completion occurs and inference stays blocked

  Scenario: Cancelled pre-prompt builder preserves authoritative state
    Given a persisted conversation above its selected compaction threshold
    When pre-prompt compaction is cancelled while its builder is running
    Then authoritative messages turns counters and durable envelopes remain unchanged
    And no provider request or completion occurs and inference stays blocked
