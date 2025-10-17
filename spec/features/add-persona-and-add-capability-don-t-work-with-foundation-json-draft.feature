@done
@foundation
@cli
@phase1
@BUG-018
Feature: add-persona and add-capability don't work with foundation.json.draft

  """
  add-persona and add-capability commands must check for foundation.json.draft first, falling back to foundation.json if draft doesn't exist. This enables AI agents to add personas/capabilities during draft-driven discovery phase. Commands use same read/write logic as update-foundation: prefer draft, maintain backward compatibility with final foundation.json. Error messages guide users to run discover-foundation when neither file exists.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. add-persona MUST check for foundation.json.draft first, then foundation.json
  #   2. add-capability MUST check for foundation.json.draft first, then foundation.json
  #   3. Commands MUST prefer draft over final foundation.json (draft takes precedence)
  #   4. Error messages MUST be helpful when neither file exists
  #
  # EXAMPLES:
  #   1. AI runs 'fspec add-persona' during discovery, draft exists, persona added to draft
  #   2. AI runs 'fspec add-capability' during discovery, draft exists, capability added to draft
  #   3. AI runs 'fspec add-persona', no draft but foundation.json exists, persona added to foundation.json (backward compatibility)
  #   4. AI runs 'fspec add-capability', neither file exists, shows helpful error with 'fspec discover-foundation' suggestion
  #
  # ========================================

  Background: User Story
    As a AI agent using draft-driven discovery
    I want to add personas and capabilities during discovery phase
    So that I can complete all foundation fields without waiting for finalization

  Scenario: Add persona to foundation.json.draft during discovery
    Given I have a foundation.json.draft file
    When I run `fspec add-persona "Developer" "Software engineers" --goal "Build features"`
    Then the command should exit with code 0
    And the output should contain "Added persona to foundation.json.draft"
    And the persona should be added to spec/foundation.json.draft
    And spec/foundation.json should not be modified

  Scenario: Add capability to foundation.json.draft during discovery
    Given I have a foundation.json.draft file
    When I run `fspec add-capability "User Authentication" "Login and registration"`
    Then the command should exit with code 0
    And the output should contain "Added capability to foundation.json.draft"
    And the capability should be added to spec/foundation.json.draft
    And spec/foundation.json should not be modified

  Scenario: Add persona to foundation.json when no draft exists (backward compatibility)
    Given I have a foundation.json file
    And I do not have a foundation.json.draft file
    When I run `fspec add-persona "User" "End users" --goal "Use the app"`
    Then the command should exit with code 0
    And the output should contain "Added persona to foundation.json"
    And the persona should be added to spec/foundation.json

  Scenario: Show helpful error when neither file exists
    Given I do not have a foundation.json.draft file
    And I do not have a foundation.json file
    When I run `fspec add-capability "Feature" "Description"`
    Then the command should exit with code 1
    And the output should contain "foundation.json not found"
    And the output should contain "fspec discover-foundation"
