@phase5
@cli
@feature-management
@modification
@medium
@unit-test
Feature: Delete Step from Scenario
  """
  Architecture notes:
  - Removes individual step from scenarios in feature files
  - Uses @cucumber/gherkin parser to locate step by scenario and step text
  - Removes complete step line
  - Preserves other steps and scenario structure
  - Validates feature file syntax after deletion
  - Supports partial step text matching for convenience

  Critical implementation requirements:
  - MUST parse Gherkin to find step location
  - MUST remove complete step line
  - MUST preserve scenario structure and other steps
  - MUST validate Gherkin syntax after deletion
  - MUST handle multiple scenarios with same step text
  - MUST provide clear error if step not found
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer refining acceptance criteria
    I want to delete individual steps from scenarios
    So that I can remove outdated or incorrect steps without manually editing
    files



  Scenario: Delete step by exact text
    Given I have a scenario "Login" with steps:
      | Given I am on the login page   |
      | When I enter valid credentials |
      | Then I should be logged in     |
      | And I should see the dashboard |
    When I run `fspec delete-step login "Login" "And I should see the dashboard"`
    Then the command should exit with code 0
    And the step "And I should see the dashboard" should be removed
    And the other 3 steps should remain
    And the feature file should be valid Gherkin
    And the output should show "Successfully deleted step from scenario 'Login'"

  Scenario: Delete Given step
    Given I have a scenario with a Given step "I am logged in"
    When I run `fspec delete-step test "Test scenario" "Given I am logged in"`
    Then the command should exit with code 0
    And the Given step should be removed
    And remaining steps should be preserved

  Scenario: Delete When step
    Given I have a scenario with steps Given, When, Then
    When I run `fspec delete-step test "Test scenario" "When I click the button"`
    Then the command should exit with code 0
    And the When step should be removed
    And the Given and Then steps should remain

  Scenario: Delete Then step
    Given I have a scenario with multiple Then steps
    When I run `fspec delete-step test "Test scenario" "Then I should see success"`
    Then the command should exit with code 0
    And only the specified Then step should be removed

  Scenario: Delete last remaining step
    Given I have a scenario with only one step "Given test"
    When I run `fspec delete-step test "Test scenario" "Given test"`
    Then the command should exit with code 0
    And the step should be removed
    And the scenario should have no steps
    And the scenario header should remain

  Scenario: Attempt to delete non-existent step
    Given I have a scenario with 3 steps
    When I run `fspec delete-step test "Test scenario" "Given non-existent step"`
    Then the command should exit with code 1
    And the output should show "Step 'Given non-existent step' not found in scenario 'Test scenario'"
    And the file should remain unchanged

  Scenario: Delete step from specific scenario when multiple scenarios exist
    Given I have two scenarios "First" and "Second"
    And both scenarios have a step "Given I am logged in"
    When I run `fspec delete-step test "First" "Given I am logged in"`
    Then the command should exit with code 0
    And the step should be removed from "First" scenario only
    And the step should remain in "Second" scenario

  Scenario: Delete step preserves indentation
    Given I have a properly indented scenario
    When I run `fspec delete-step test "Test scenario" "When I click submit"`
    Then the command should exit with code 0
    And the remaining steps should preserve proper indentation
    And the file should remain properly formatted

  Scenario: Delete step with special characters in text
    Given I have a step "Given I can't access @admin features"
    When I run `fspec delete-step test "Test scenario" "Given I can't access @admin features"`
    Then the command should exit with code 0
    And the step should be removed

  Scenario: Handle scenario not found
    Given I have a feature file "login.feature"
    When I run `fspec delete-step login "Non-existent scenario" "Given test"`
    Then the command should exit with code 1
    And the output should show "Scenario 'Non-existent scenario' not found"
    And the file should remain unchanged

  Scenario: Handle feature file with invalid syntax
    Given I have a feature file with syntax errors
    When I run `fspec delete-step broken "Some scenario" "Given test"`
    Then the command should exit with code 1
    And the output should show "Invalid Gherkin syntax"
    And the file should remain unchanged
