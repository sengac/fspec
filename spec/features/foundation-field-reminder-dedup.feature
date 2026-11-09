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

  Scenario: The projectType reminder asks the AI to determine the type and verify it with the human
    Given a project root whose spec/foundation.json.draft has project.name and project.vision filled and project.projectType "[QUESTION: What type of project is this?]"
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the result systemReminder contains 'Field 3/8: project.projectType'
    And the result systemReminder contains 'Analyze codebase to determine project type. Verify with human.'
    And the result systemReminder lists the example project types cli-tool, web-app and library
    And the result systemReminder contains 'Run: fspec update-foundation projectType "<type>"'

  Scenario: A detected projectType is shown in its reminder for the human to verify
    Given a project root whose spec/foundation.json.draft has project.name and project.vision filled and project.projectType "[DETECTED: web-app]"
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the result systemReminder contains 'Field 3/8: project.projectType'
    And the result systemReminder contains '[DETECTED: web-app] Analyze codebase to determine project type. Verify with human.'
    And the result systemReminder contains 'Run: fspec update-foundation projectType "<type>"'

  Scenario: The primary problem reminder asks for the USER perspective
    Given a project root whose spec/foundation.json.draft has the project fields filled and problemSpace.primaryProblem.title "[QUESTION: What problem does this solve?]"
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the result systemReminder contains 'Field 4/8: problemSpace.primaryProblem.title'
    And the result systemReminder contains 'CRITICAL: Think from USER perspective. WHO uses this (persona)?'
    And the result systemReminder contains 'WHAT problem do THEY face? WHY do they need this solution?'
    And the result systemReminder contains 'Requires title, description, impact.'

  Scenario: The capabilities reminder asks for what users can do, not how
    Given a project root whose spec/foundation.json.draft has every field before solutionSpace.capabilities filled and a capability "[QUESTION: What can users DO?]"
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the result systemReminder contains 'Field 7/8: solutionSpace.capabilities'
    And the result systemReminder contains 'List 3-7 high-level abilities users have. Focus on WHAT not HOW.'
    And the result systemReminder contains 'Example: "Spec Validation" (WHAT), NOT "Uses Cucumber parser" (HOW)'
    And the result systemReminder contains 'Run: fspec add-capability "Capability Name" "Capability Description"'

  Scenario: The personas reminder asks for every user type with goals and pain points
    Given a project root whose spec/foundation.json.draft has every field but personas filled and a persona "[QUESTION: Who uses this?]"
    When I dispatch update-foundation with section='projectName' and content='fspec'
    Then the result systemReminder contains 'Field 8/8: personas'
    And the result systemReminder contains 'Identify ALL user types from interactions.'
    And the result systemReminder contains 'CLI tools: who runs commands? Web apps: who uses UI + who calls API?'
    And the result systemReminder contains 'Ask human about goals and pain points.'
    And the result systemReminder contains 'Run: fspec add-persona "Persona Name" "Persona Description" --goal "Primary goal"'
