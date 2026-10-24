@done
@CMPCT-072
@compaction
@agent-core
@phase-1
@validation
Feature: Durable DAG preservation
  """
  All producers supply an authoritative validation context before promotion.
  Durable D2 node identities and payload bytes remain unchanged; required file
  identities and reminders are validated, never repaired after rejection.
  The shared validator is independent of active-history message counts.
  This component specification verifies the shared contract, not route execution.
  Runtime installation and cold replay are complementary integration evidence in
  runtime-durable-dag-installation.feature and cold-resume-compaction-acceptance-gate.feature.
  """

  Background: User Story
    As an agent preserving decisions across compactions
    I want every candidate to retain authoritative durable content
    So that no supported route silently drops durable memory or file awareness

  Scenario: Preserve durable payloads and file awareness in the shared validator
    Given authoritative durable nodes file identities and required reminders
    When a complete candidate is validated against the shared preservation contract
    Then complete candidates preserve the exact durable payload and identities

  Scenario: Reject missing or rewritten durable content
    Given authoritative durable nodes file identities and required reminders
    When a candidate omits or rewrites required content
    Then the whole candidate is rejected without fragment repair

  Scenario: Reject malformed or empty required file metadata
    Given an authoritative nonempty file awareness contract
    When a candidate supplies malformed empty or duplicate file metadata
    Then validation fails without inventing file awareness

  Scenario: Reject inaccessible or invalid authoritative source
    Given an invalid existing marked DAG
    When a candidate requests a fresh rebuild instead
    Then validation returns a typed invalid source failure

  Scenario: Assemble missing file metadata deterministically before validation
    Given authoritative file changes and a candidate with no file metadata
    When the selected protocol prepares the complete candidate
    Then sorted file metadata is assembled once and validated without changing durable payloads

  Scenario: Unavailable originals reject source capture and promotion
    Given a persisted source whose immutable originals become unavailable
    When a builder captures the source or a captured candidate requests promotion
    Then both operations fail explicitly before admission without changing the durable boundary

  Scenario: Empty authoritative reminders cannot authorize preservation
    Given authoritative reminders containing an empty or whitespace-only obligation
    When an otherwise complete candidate is validated
    Then validation returns a typed invalid source failure rather than vacuous preservation

  Scenario: Cached originals cannot conceal missing durable evidence
    Given immutable originals remain cached after their durable storage disappears
    When source capture promotion or replay checks the persisted evidence
    Then every operation rejects the missing originals before admission or publication
    And the committed manifest bytes remain unchanged
