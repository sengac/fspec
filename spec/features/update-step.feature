@phase5
@cli
@feature-management
@modification
@medium
@unit-test
Feature: Update Step in Scenario
  """
  Architecture notes:
  - Modifies step text or keyword in scenarios
  - Uses @cucumber/gherkin parser to locate step by scenario and current text
  - Updates step while preserving scenario structure
  - Validates feature file syntax after update
  - Supports changing step keyword (Given -> When, etc.)

  Critical implementation requirements:
  - MUST parse Gherkin to find step location
  - MUST update step text or keyword
  - MUST preserve other steps and scenario structure
  - MUST validate Gherkin syntax after update
  - MUST handle multiple steps with same text in different scenarios
  - MUST provide clear error if step not found
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer refining acceptance criteria
    I want to update step text or keywords
    So that I can improve clarity and accuracy without manually editing files



  Scenario: Update step text only
    Given I have a step "Given I am logged in"
    When I run `fspec update-step login "Login test" "Given I am logged in" --text="Given I am authenticated"`
    Then the command should exit with code 0
    And the step should be "Given I am authenticated"
    And the keyword should remain "Given"
    And other steps should be preserved
    And the feature file should be valid Gherkin
    And the output should show "Successfully updated step"

  Scenario: Update step keyword only
    Given I have a step "Given I am on the page"
    When I run `fspec update-step test "Test scenario" "Given I am on the page" --keyword="When"`
    Then the command should exit with code 0
    And the step should be "When I am on the page"
    And the step text should remain "I am on the page"

  Scenario: Update both step text and keyword
    Given I have a step "Given I click the button"
    When I run `fspec update-step test "Test scenario" "Given I click the button" --keyword="When" --text="When I submit the form"`
    Then the command should exit with code 0
    And the step should be "When I submit the form"
    And the old step should not exist

  Scenario: Update step in specific scenario when multiple scenarios have same step
    Given I have two scenarios "First" and "Second"
    And both scenarios have step "Given I am logged in"
    When I run `fspec update-step test "First" "Given I am logged in" --text="Given I have authenticated"`
    Then the command should exit with code 0
    And the step in "First" scenario should be updated
    And the step in "Second" scenario should remain unchanged

  Scenario: Update step preserves position in scenario
    Given I have a scenario with steps in order: "Given A", "When B", "Then C"
    When I run `fspec update-step test "Test scenario" "When B" --text="When B updated"`
    Then the command should exit with code 0
    And the steps should remain in order: "Given A", "When B updated", "Then C"

  Scenario: Update step with special characters
    Given I have a step "Given I can login"
    When I run `fspec update-step test "Test scenario" "Given I can login" --text="Given I can't login with @invalid credentials"`
    Then the command should exit with code 0
    And the step should contain apostrophe and special characters
    And the feature file should be valid Gherkin

  Scenario: Attempt to update non-existent step
    Given I have a scenario with 3 steps
    When I run `fspec update-step test "Test scenario" "Given non-existent step" --text="New text"`
    Then the command should exit with code 1
    And the output should show "Step 'Given non-existent step' not found"
    And the file should remain unchanged

  Scenario: Update And step to Given step
    Given I have a step "And I enter password"
    When I run `fspec update-step test "Test scenario" "And I enter password" --keyword="Given"`
    Then the command should exit with code 0
    And the step should be "Given I enter password"

  Scenario: Update step preserves indentation
    Given I have a properly indented step
    When I run `fspec update-step test "Test scenario" "Given I am on page" --text="Given I am on the login page"`
    Then the command should exit with code 0
    And the updated step should maintain proper indentation

  Scenario: Handle scenario not found
    Given I have a feature file
    When I run `fspec update-step test "Non-existent scenario" "Given test" --text="New text"`
    Then the command should exit with code 1
    And the output should show "Scenario 'Non-existent scenario' not found"
    And the file should remain unchanged

  Scenario: Require at least one update option
    Given I have a step "Given test"
    When I run `fspec update-step test "Test scenario" "Given test"` without --text or --keyword
    Then the command should exit with code 1
    And the output should show "No updates specified. Use --text and/or --keyword"

  Scenario: Handle feature file with invalid syntax
    Given I have a feature file with syntax errors
    When I run `fspec update-step broken "Some scenario" "Given test" --text="New text"`
    Then the command should exit with code 1
    And the output should show "Invalid Gherkin syntax"
    And the file should remain unchanged
