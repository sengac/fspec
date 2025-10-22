@cli
@feature-management
@modification
@medium
@unit-test
Feature: Delete Scenario from Feature File
  """
  Architecture notes:
  - Removes individual scenario from feature files
  - Uses @cucumber/gherkin parser to locate scenario by name
  - Removes complete scenario block including all steps
  - Preserves other scenarios and feature structure
  - Validates feature file syntax after deletion
  - Supports both exact scenario names and fuzzy matching

  Critical implementation requirements:
  - MUST parse Gherkin to find scenario location
  - MUST remove complete scenario block (header + all steps)
  - MUST preserve feature structure and other scenarios
  - MUST validate Gherkin syntax after deletion
  - MUST handle scenarios with same names in different features
  - MUST provide clear error if scenario not found
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing evolving requirements
    I want to delete obsolete scenarios from feature files
    So that I can keep specifications clean and relevant without manually
    editing files

  Scenario: Delete scenario by exact name
    Given I have a feature file "login.feature" with 3 scenarios
    And one scenario is named "Invalid password"
    When I run `fspec delete-scenario login "Invalid password"`
    Then the command should exit with code 0
    And the scenario "Invalid password" should be removed
    And the other 2 scenarios should remain
    And the feature file should be valid Gherkin
    And the output should show "Successfully deleted scenario 'Invalid password' from login.feature"

  Scenario: Delete scenario from nested directory
    Given I have a feature file "spec/features/auth/login.feature"
    And the feature has a scenario "Test scenario"
    When I run `fspec delete-scenario spec/features/auth/login.feature "Test scenario"`
    Then the command should exit with code 0
    And the scenario should be removed
    And the file should remain valid

  Scenario: Delete last remaining scenario
    Given I have a feature file with only one scenario "Only scenario"
    When I run `fspec delete-scenario test "Only scenario"`
    Then the command should exit with code 0
    And the scenario should be removed
    And the feature should have no scenarios
    And the background and feature header should remain intact

  Scenario: Attempt to delete non-existent scenario
    Given I have a feature file "login.feature" with scenarios
    When I run `fspec delete-scenario login "Non-existent scenario"`
    Then the command should exit with code 1
    And the output should show "Scenario 'Non-existent scenario' not found"
    And the file should remain unchanged

  Scenario: Delete scenario preserves background
    Given I have a feature file with a Background section
    And the feature has a scenario "Test scenario"
    When I run `fspec delete-scenario test "Test scenario"`
    Then the Background section should remain intact
    And the scenario should be removed

  Scenario: Delete scenario preserves formatting
    Given I have a formatted feature file
    And the feature has a scenario "Test scenario"
    When I run `fspec delete-scenario test "Test scenario"`
    Then the remaining file should preserve indentation
    And the remaining file should preserve spacing
    And the file should remain properly formatted

  Scenario: Delete scenario with special characters in name
    Given I have a scenario named "User can't login with @special chars"
    When I run `fspec delete-scenario test "User can't login with @special chars"`
    Then the command should exit with code 0
    And the scenario should be removed

  Scenario: Handle feature file with invalid syntax
    Given I have a feature file "broken.feature" with syntax errors
    When I run `fspec delete-scenario broken "Some scenario"`
    Then the command should exit with code 1
    And the output should show "Invalid Gherkin syntax"
    And the file should remain unchanged
