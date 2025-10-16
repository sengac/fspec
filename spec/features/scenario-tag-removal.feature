@phase1
@critical
@bug-fix
@tag-management
@cli
@BUG-009
Feature: Scenario Tag Removal
  """
  Uses Edit tool to remove scenario-level tags from feature files. Must handle multiple tags on same scenario, preserve formatting, and validate Gherkin syntax after removal. Critical for tag compliance where work unit IDs must be at feature level only.
  """

  Background: User Story
    As a developer using fspec
    I want to remove scenario-level tags that should be at feature level
    So that tag validation passes and work unit tags are correctly positioned

  Scenario: Remove single tag from scenario
    Given I have a feature file with a scenario that has a single tag @COV-010
    When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
    Then the tag @COV-010 should be removed from the scenario line
    And the command should exit with code 0
    And the feature file should have valid Gherkin syntax

  Scenario: Remove one of multiple tags from scenario
    Given I have a feature file with a scenario that has tags @COV-010 @wip @critical
    When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
    Then the tag @COV-010 should be removed from the scenario
    And the tags @wip and @critical should remain on the scenario
    And the command should exit with code 0

  Scenario: Remove tag from scenario when scenario not found
    Given I have a feature file without a scenario named "Nonexistent Scenario"
    When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Nonexistent Scenario" @COV-010'
    Then the command should display a warning message
    And the command should exit with code 0 (idempotent behavior)

  Scenario: Remove tag that doesn't exist on scenario
    Given I have a feature file with a scenario that has only @wip tag
    When I run 'fspec remove-tag-from-scenario spec/features/test.feature "Login scenario" @COV-010'
    Then the command should succeed (idempotent behavior)
    And the command should exit with code 0
    And the @wip tag should remain on the scenario
