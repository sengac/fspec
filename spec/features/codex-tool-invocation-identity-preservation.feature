@phase-1
@persistence
@providers
@provider-abstraction
@regression
Feature: Preserve distinct tool item and invocation identities
  """
  Architecture notes:
  Tool item identities remain unchanged for UI correlation. Optional invocation
  identities accompany assistant calls and user tool outcomes through stream
  events and persisted envelopes. Legacy single-identity callers remain valid.
  """

  Scenario: Serialized tool blocks retain explicit invocation identities
    Given assistant tool calls and tool outcomes have distinct item and invocation identities
    When their persisted content blocks are deserialized and serialized
    Then both blocks retain their original item and invocation identities

  Scenario: Legacy tool blocks remain compatible without invocation identities
    Given legacy tool calls and tool outcomes have only item identities
    When their persisted content blocks are deserialized and serialized
    Then no invocation identity field is added

  Scenario: Streamed tool identities survive persistence and cold restore
    Given a streamed assistant tool call and cancellation outcome have distinct identities
    When their events are persisted and loaded into a fresh session
    Then the restored call and outcome retain both original identities
    And the next provider request replays the original call and cancellation exactly once
