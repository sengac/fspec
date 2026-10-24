@done
@agent-core
@session-management
@persistence
@BUG-249
Feature: Transcript semantic admission
  """
  A session-wide durable ordinal orders acknowledgements independently of wall
  clocks. Missing interior session or request sequences fail explicitly.
  Durable admission requires a readable truthful tool lifecycle, including the
  candidate delta. Retries of an existing immutable record do not apply it twice.
  """

  Background: User Story
    As an agent recovering a persisted session
    I want authentic history and explicit persistence failures
    So that recovery preserves evidence without inventing outcomes

  Scenario: Conflicting record identities are rejected before acknowledgement
    Given an acknowledged transcript record with a stable identity
    When another sequence or request reuses that record identity
    Then admission rejects the conflict without changing the recoverable transcript

  Scenario: Corrupt acknowledged history prevents further admission
    Given an acknowledged transcript record whose stored bytes are corrupted
    When another delta is submitted for durable admission
    Then admission fails without publishing another record or changing the corruption evidence

  Scenario: Invalid tool lifecycle transitions are rejected before acknowledgement
    Given missing-request starts duplicate starts and cross-attempt results
    When each invalid tool transition is submitted for durable admission
    Then admission rejects every invalid lifecycle without changing readable history

  Scenario: Persisted invalid lifecycles prevent recovery and further admission
    Given persisted records containing an invalid tool lifecycle
    When the store is reopened and another delta is submitted
    Then recovery and admission report the lifecycle failure without changing evidence

  Scenario: Valid tool lifecycle retries remain idempotent
    Given an acknowledged requested started and completed tool lifecycle
    When each original record is retried after reopening the completed lifecycle
    Then all retries retain the exact original records and one authentic completed outcome
