@done
@BUG-243
@phase-1
@persistence
@context-management
Feature: Recover verified native provider state
  """
  Replay validates content-addressed native state and its manifest prefix.
  Missing or corrupt referenced state prevents continuation. Genuine originals
  retain their exact evidence errors and native requests preserve item identity.
  """

  Background: User Story
    As an agent resuming native inference
    I want exact durable checkpoints and explicit publication failures
    So that recovery cannot silently replace or reset provider lineage

  Scenario: Cold continuation preserves native reasoning and tool identity
    Given real persisted native reasoning and paired tool invocation outcomes
    When a cold provider restores and sends its next physical request
    Then model protocol generation reasoning and authentic tool identity survive
    And no lineage reset or alternate request is used

  Scenario: Reject corrupt originals and incompatible state without fallback
    Given malformed genuine dialogue oversized dialogue and a mismatched state anchor
    When originals and provider replay are requested
    Then genuine dialogue returns its exact typed missing or oversized failure
    And incompatible state prevents replay without legacy lineage fallback

  Scenario: Preserve prior state across checkpoint publication failures
    Given a committed checkpoint and an unavailable checkpoint promotion destination
    When a new checkpoint publication fails and is retried after recovery
    Then the failure is explicit and the previous committed state remains recoverable
    And retry selects the exact new native checkpoint without adding conversation records
