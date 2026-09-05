@done
@querying
@rust
@cli
@foundation-management
@DISC-003
Feature: field reminder text is owned by the shared guidance module
  """
  DISC-003 (part 4/5): the duplicated field-scan/reminder code is deleted from discover_foundation.rs and update_foundation.rs and lives in the single shared module foundation/guidance.rs; the reminder keeps its asserted substrings and appends Examples.
  """

  Background: User Story
    As a AI agent
    I want to discover and fill the project foundation with clear per-step guidance and progress
    So that I always know what fields remain, what to do next, and what content is appropriate, without guessing or re-reading raw JSON

  Scenario: the field reminder text comes from the shared guidance module with examples
    Given a project root whose spec/foundation.json.draft has only project.name filled
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the dispatcher returns success=true
    And the result systemReminder contains 'Field 2/8: project.vision (elevator pitch)'
    And the result systemReminder contains 'Run: fspec update-foundation projectVision'
    And the result systemReminder contains an 'Examples:' section
    And update_foundation.rs and discover_foundation.rs no longer define their own scan or field-reminder functions
