@phase-1
@session-management
@providers
@compaction
@regression
Feature: Model switches preserve the durable selected identity
  A successful model switch must persist the same provider, profile, and model
  identity used by the runtime so subsequent compaction captures the selected
  request identity rather than the session's creation-time identity.
  Rejected switches preserve the request environment as well as the selected
  identity. Publication is the boundary for adopting a selection: an unavailable
  durability acknowledgement after publication cannot leave the runtime behind.

  Scenario: Switching to a profile preserves its identity through compaction
    Given a persisted session with an original message and a different selected model
    When the session switches to an OpenAI profile model
    Then its durable provider and runtime selection include the same profile and model
    And a captured compaction can commit and survive cold restoration

  Scenario: Switching to a cloud model preserves its identity through compaction
    Given a persisted session with an original message and a different selected model
    When the session switches to a cloud model from another provider
    Then its durable provider and runtime selection include the same provider and model
    And a captured compaction can commit and survive cold restoration

  Scenario: Rejected model selection preserves the prior durable identity
    Given a persisted session with an original message and a different selected model
    When the session selects a missing profile
    Then the switch fails without changing the durable identity or runtime selection and limits

  Scenario: A busy session preserves the prior durable identity
    Given a persisted session whose inner session is locked
    When the session attempts to switch model
    Then the switch fails without changing the durable identity or runtime selection and limits
    And the prior request environment remains unchanged

  Scenario: Published identity is adopted despite a lost durability acknowledgement
    Given a persisted session whose manifest can be published but not acknowledged
      """
      Architecture: once the selected identity is published, runtime adoption
      follows publication rather than the durability acknowledgement result.
      """
    When the session switches to an OpenAI profile model
    Then the switch succeeds with matching durable and runtime selection and limits
    And the selected profile request environment is retained

  Scenario: Failed profile preparation preserves the prior request environment
    Given a persisted session and a saved profile for an unsupported provider
    When the session attempts to switch to that profile
    Then the switch fails without changing the durable identity or runtime selection and limits
    And the prior request environment remains unchanged

  Scenario: Durable identity write failure preserves the prior runtime selection
    Given a persisted session whose manifest is readable but cannot be updated
    When the session attempts to switch model
    Then the switch fails without changing the durable identity or runtime selection and limits

  Scenario: Switching an existing committed identity is rejected without rewriting ancestry
    Given a persisted session with a committed compaction generation
    When the session attempts to switch to a different selected identity
    Then the switch fails without changing the durable identity or runtime selection and limits
    And the prior request environment remains unchanged
    And the committed generation remains valid
