@done
@foundation
@cli
@phase1
@FEAT-014
Feature: Add remove-persona and remove-capability commands
  """
  New commands remove-persona and remove-capability mirror add-persona/add-capability with same draft-first priority logic. Commands find matching persona/capability by exact name (case-sensitive), remove from array, write updated file. If not found, show error listing available names for user guidance. Follows same pattern as add commands: check draft first with fs.access, fall back to foundation.json, maintain backward compatibility.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. remove-persona MUST work with both foundation.json and foundation.json.draft (same file priority as add-persona)
  #   2. remove-capability MUST work with both foundation.json and foundation.json.draft (same file priority as add-capability)
  #   3. Remove commands MUST match by exact name (case-sensitive)
  #   4. If persona/capability not found, show error with list of available names
  #
  # EXAMPLES:
  #   1. User runs 'fspec remove-persona Developer', persona removed from draft
  #   2. User runs 'fspec remove-capability "Mind Mapping"', capability removed from foundation.json
  #   3. User runs 'fspec remove-persona NonExistent', shows error listing available personas
  #
  # ========================================
  Background: User Story
    As a user managing foundation document
    I want to remove unwanted personas and capabilities
    So that I can clean up placeholders without manually editing JSON

  Scenario: Remove persona from draft by name
    Given I have a foundation.json.draft file
    And the draft contains a persona named "Developer"
    When I run `fspec remove-persona "Developer"`
    Then the command should exit with code 0
    And the output should contain "Removed persona \"Developer\""
    And the persona "Developer" should be removed from spec/foundation.json.draft

  Scenario: Remove capability from foundation.json by name
    Given I have a foundation.json file
    And the file contains a capability named "Mind Mapping"
    When I run `fspec remove-capability "Mind Mapping"`
    Then the command should exit with code 0
    And the output should contain "Removed capability \"Mind Mapping\""
    And the capability "Mind Mapping" should be removed from spec/foundation.json

  Scenario: Show error when persona not found
    Given I have a foundation.json.draft file
    And the draft contains personas "Developer" and "Researcher"
    When I run `fspec remove-persona "NonExistent"`
    Then the command should exit with code 1
    And the output should contain "Persona \"NonExistent\" not found"
    And the output should contain "Available personas: Developer, Researcher"
