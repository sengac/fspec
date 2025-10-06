@phase3
@cli
@feature-management
@modification
@medium
@unit-test
Feature: Add Step to Existing Scenario
  """
  Architecture notes:
  - Reads existing feature file and parses with @cucumber/gherkin
  - Validates feature file exists and has valid syntax before modification
  - Locates the specified scenario by name
  - Appends step to the end of the scenario
  - Supports Given/When/Then/And/But step types
  - Preserves existing formatting and indentation
  - Does NOT format after insertion (user can run fspec format separately)
  - Validates Gherkin syntax after insertion to ensure file remains valid

  Critical implementation requirements:
  - MUST validate feature file exists before modification
  - MUST parse existing file to ensure valid Gherkin syntax
  - MUST find scenario by exact name match
  - MUST append step to the end of the scenario (before next scenario)
  - MUST use proper indentation matching existing steps
  - MUST support all step types: Given, When, Then, And, But
  - MUST validate result is valid Gherkin after insertion
  - Exit code 0 for success, 1 for errors

  References:
  - Gherkin spec: https://cucumber.io/docs/gherkin/reference
  - Parser: @cucumber/gherkin
  """

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to add steps to existing scenarios incrementally
    So that I can build acceptance criteria step by step without manual editing



  Scenario: Add Given step to scenario
    Given I have a feature file "spec/features/login.feature" with scenario "User login"
    When I run `fspec add-step login "User login" given "I am on the login page"`
    Then the command should exit with code 0
    And the scenario should contain "Given I am on the login page"
    And the file should remain valid Gherkin syntax

  Scenario: Add When step to scenario
    Given I have a feature file with scenario "Submit form"
    When I run `fspec add-step my-feature "Submit form" when "I click the submit button"`
    Then the scenario should contain "When I click the submit button"
    And the step should be properly indented

  Scenario: Add Then step to scenario
    Given I have a feature file with scenario "Validation"
    When I run `fspec add-step my-feature "Validation" then "I should see a success message"`
    Then the scenario should contain "Then I should see a success message"

  Scenario: Add And step to scenario
    Given I have a feature file with scenario "Multiple steps"
    When I run `fspec add-step my-feature "Multiple steps" and "I fill in the email field"`
    Then the scenario should contain "And I fill in the email field"

  Scenario: Add But step to scenario
    Given I have a feature file with scenario "Edge case"
    When I run `fspec add-step my-feature "Edge case" but "I should not see an error"`
    Then the scenario should contain "But I should not see an error"

  Scenario: Add multiple steps to same scenario
    Given I have a feature file with scenario "Complex workflow" with 1 step
    When I run `fspec add-step my-feature "Complex workflow" when "I enter my password"`
    And I run `fspec add-step my-feature "Complex workflow" then "I should be logged in"`
    Then the scenario should have 3 steps total
    And all steps should be in the order they were added
    And the file should remain valid Gherkin syntax

  Scenario: Preserve indentation when adding step
    Given I have a scenario with 4-space indented steps
    When I run `fspec add-step my-feature "Test" given "new step"`
    Then the new step should have the same indentation as existing steps
    And the indentation should be 4 spaces from feature level

  Scenario: Handle feature file not found
    Given there is no feature file "spec/features/missing.feature"
    When I run `fspec add-step missing "Scenario" given "step"`
    Then the command should exit with code 1
    And the output should show error "Feature file not found"
    And the output should suggest using create-feature command

  Scenario: Handle scenario not found
    Given I have a feature file "spec/features/test.feature"
    And the file does not have a scenario named "Missing scenario"
    When I run `fspec add-step test "Missing scenario" given "step"`
    Then the command should exit with code 1
    And the output should show error that scenario was not found
    And the output should list available scenarios
    And the file should not be modified

  Scenario: Handle invalid feature file syntax
    Given I have a feature file with invalid Gherkin syntax
    When I run `fspec add-step broken "Scenario" given "step"`
    Then the command should exit with code 1
    And the output should show error about invalid Gherkin syntax
    And the output should suggest running validate command first
    And the file should not be modified

  Scenario: Handle invalid step type
    Given I have a feature file with scenario "Test"
    When I run `fspec add-step my-feature "Test" invalid "step text"`
    Then the command should exit with code 1
    And the output should show error about invalid step type
    And the output should list valid step types: given, when, then, and, but

  Scenario: Use feature name without extension
    Given I have a feature file "spec/features/user-auth.feature" with scenario "Login"
    When I run `fspec add-step user-auth "Login" given "I am logged out"`
    Then the command should exit with code 0
    And the scenario should contain the new step

  Scenario: Use full file path
    Given I have a feature file "spec/features/checkout.feature" with scenario "Payment"
    When I run `fspec add-step spec/features/checkout.feature "Payment" when "I enter card details"`
    Then the command should exit with code 0
    And the scenario should contain the new step

  Scenario: Case-insensitive step type
    Given I have a feature file with scenario "Test"
    When I run `fspec add-step my-feature "Test" GIVEN "step text"`
    Then the command should exit with code 0
    And the step should be formatted as "Given step text"

  Scenario: Handle scenario with data table
    Given I have a scenario with a data table at the end
    When I run `fspec add-step my-feature "Scenario" and "another step"`
    Then the new step should be added before the data table
    And the data table should remain at the end
    And the file should remain valid Gherkin syntax

  Scenario: Handle scenario with doc string
    Given I have a scenario with a doc string at the end
    When I run `fspec add-step my-feature "Scenario" and "another step"`
    Then the new step should be added before the doc string
    And the doc string should remain at the end
    And the file should remain valid Gherkin syntax
