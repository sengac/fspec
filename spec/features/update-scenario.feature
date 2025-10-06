@phase5
@cli
@feature-management
@modification
@medium
@unit-test
Feature: Update Scenario Name
  """
  Architecture notes:
  - Renames scenarios in feature files
  - Uses @cucumber/gherkin parser to locate scenario by current name
  - Updates scenario header while preserving all steps and structure
  - Validates feature file syntax after update
  - Handles duplicate scenario name detection

  Critical implementation requirements:
  - MUST parse Gherkin to find scenario location
  - MUST update only the scenario name line
  - MUST preserve all steps, tags, and scenario structure
  - MUST validate Gherkin syntax after update
  - MUST detect duplicate scenario names
  - MUST provide clear error if scenario not found
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer refining specifications
    I want to rename scenarios to better reflect their intent
    So that acceptance criteria remain clear and accurate without manually
    editing files



  Scenario: Rename scenario with simple name
    Given I have a scenario named "Old scenario name"
    When I run `fspec update-scenario login "Old scenario name" "New scenario name"`
    Then the command should exit with code 0
    And the scenario name should be "New scenario name"
    And all steps should be preserved
    And the feature file should be valid Gherkin
    And the output should show "Successfully renamed scenario to 'New scenario name'"

  Scenario: Rename scenario preserves steps
    Given I have a scenario "Login test" with 5 steps
    When I run `fspec update-scenario test "Login test" "User authentication test"`
    Then the command should exit with code 0
    And the scenario name should be updated
    And all 5 steps should remain unchanged
    And step order should be preserved

  Scenario: Rename scenario preserves tags
    Given I have a scenario with tags @critical @high
    And the scenario is named "Payment processing"
    When I run `fspec update-scenario test "Payment processing" "Process payment transaction"`
    Then the command should exit with code 0
    And the scenario tags should be preserved
    And the scenario name should be updated

  Scenario: Rename scenario when multiple scenarios exist
    Given I have a feature with scenarios "First", "Second", "Third"
    When I run `fspec update-scenario test "Second" "Middle scenario"`
    Then the command should exit with code 0
    And the "First" scenario should remain unchanged
    And the "Second" scenario should be renamed to "Middle scenario"
    And the "Third" scenario should remain unchanged

  Scenario: Attempt to rename to existing scenario name
    Given I have scenarios "Login test" and "Registration test"
    When I run `fspec update-scenario test "Login test" "Registration test"`
    Then the command should exit with code 1
    And the output should show "Scenario 'Registration test' already exists"
    And the file should remain unchanged

  Scenario: Attempt to rename non-existent scenario
    Given I have a feature file with existing scenarios
    When I run `fspec update-scenario test "Non-existent scenario" "New name"`
    Then the command should exit with code 1
    And the output should show "Scenario 'Non-existent scenario' not found"
    And the file should remain unchanged

  Scenario: Rename scenario with special characters
    Given I have a scenario named "User can login"
    When I run `fspec update-scenario test "User can login" "User can't login with invalid credentials"`
    Then the command should exit with code 0
    And the scenario name should contain apostrophe and special characters
    And the feature file should be valid Gherkin

  Scenario: Rename scenario preserves indentation
    Given I have a properly indented scenario
    When I run `fspec update-scenario test "Old name" "New name"`
    Then the command should exit with code 0
    And the scenario header should maintain proper indentation
    And all steps should maintain their indentation

  Scenario: Handle feature file with invalid syntax
    Given I have a feature file with syntax errors
    When I run `fspec update-scenario broken "Some scenario" "New name"`
    Then the command should exit with code 1
    And the output should show "Invalid Gherkin syntax"
    And the file should remain unchanged

  Scenario: Rename scenario from nested directory
    Given I have a feature file "spec/features/auth/login.feature"
    And the feature has a scenario "Login flow"
    When I run `fspec update-scenario spec/features/auth/login.feature "Login flow" "Authentication workflow"`
    Then the command should exit with code 0
    And the scenario should be renamed
    And the file should remain valid
