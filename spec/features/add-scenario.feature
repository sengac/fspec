@cli
@feature-management
@modification
@medium
@unit-test
Feature: Add Scenario to Existing Feature File
  """
  Architecture notes:
  - Reads existing feature file and parses with @cucumber/gherkin
  - Validates feature file exists and has valid syntax before modification
  - Inserts new scenario at the end of the feature (before any existing Scenario


  Outline)
  - Scenario template includes Given/When/Then placeholders
  - Preserves existing formatting and structure
  - Does NOT format after insertion (user can run fspec format separately)
  - Validates Gherkin syntax after insertion to ensure file remains valid

  Critical implementation requirements:
  - MUST validate feature file exists before modification
  - MUST parse existing file to ensure valid Gherkin syntax
  - MUST preserve all existing content (tags, background, scenarios)
  - MUST insert scenario in correct location (after other scenarios, before
  Scenario Outline if present)
  - MUST use proper indentation (2 spaces)
  - MUST create scenario with Given/When/Then placeholders
  - MUST validate result is valid Gherkin after insertion
  - Exit code 0 for success, 1 for errors

  References:
  - Gherkin spec: https://cucumber.io/docs/gherkin/reference
  - Parser: @cucumber/gherkin
  """

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to add new scenarios to existing feature files
    So that I can incrementally build acceptance criteria without manual editing

  Scenario: Add scenario to feature file with template
    Given I have a feature file "spec/features/login.feature"
    When I run `fspec add-scenario login "Successful login with valid credentials"`
    Then the command should exit with code 0
    And the file should contain a new scenario named "Successful login with valid credentials"
    And the scenario should have Given/When/Then placeholders
    And the file should remain valid Gherkin syntax

  Scenario: Add scenario with feature name (without .feature extension)
    Given I have a feature file "spec/features/user-auth.feature"
    When I run `fspec add-scenario user-auth "Password reset"`
    Then the command should exit with code 0
    And the file should contain a new scenario named "Password reset"

  Scenario: Add scenario with full file path
    Given I have a feature file "spec/features/shopping-cart.feature"
    When I run `fspec add-scenario spec/features/shopping-cart.feature "Add item to cart"`
    Then the command should exit with code 0
    And the file should contain a new scenario named "Add item to cart"

  Scenario: Add multiple scenarios to same feature
    Given I have a feature file "spec/features/payment.feature" with 1 scenario
    When I run `fspec add-scenario payment "Credit card payment"`
    And I run `fspec add-scenario payment "PayPal payment"`
    Then the file should contain 3 scenarios total
    And all scenarios should be in the order they were added
    And the file should remain valid Gherkin syntax

  Scenario: Preserve existing content when adding scenario
    Given I have a feature file with tags, background, and existing scenarios
    When I run `fspec add-scenario my-feature "New scenario"`
    Then all existing tags should be preserved
    And the background section should be preserved
    And all existing scenarios should be preserved
    And the new scenario should be added at the end

  Scenario: Insert scenario before Scenario Outline if present
    Given I have a feature file with 2 scenarios and 1 Scenario Outline
    When I run `fspec add-scenario my-feature "New scenario"`
    Then the new scenario should be inserted after the 2 existing scenarios
    And the new scenario should be before the Scenario Outline
    And the file should remain valid Gherkin syntax

  Scenario: Handle feature file not found
    Given there is no feature file "spec/features/missing.feature"
    When I run `fspec add-scenario missing "New scenario"`
    Then the command should exit with code 1
    And the output should show error "Feature file not found"
    And the output should suggest using create-feature command

  Scenario: Handle invalid feature file syntax
    Given I have a feature file "spec/features/broken.feature" with invalid syntax
    When I run `fspec add-scenario broken "New scenario"`
    Then the command should exit with code 1
    And the output should show error about invalid Gherkin syntax
    And the output should suggest running validate command first
    And the file should not be modified

  Scenario: Handle duplicate scenario name
    Given I have a feature file with scenario "Login with email"
    When I run `fspec add-scenario my-feature "Login with email"`
    Then the command should exit with code 0
    And the output should show warning about duplicate scenario name
    And the new scenario should still be added

  Scenario: Use proper indentation in added scenario
    Given I have a feature file with 2-space indentation
    When I run `fspec add-scenario my-feature "New scenario"`
    Then the new scenario should use 2-space indentation
    And all steps should be indented 4 spaces from feature level
    And the indentation should match existing scenarios

  Scenario: Scenario template includes placeholder steps
    Given I have a feature file "spec/features/test.feature"
    When I run `fspec add-scenario test "Test scenario"`
    Then the scenario should contain "Given [precondition]"
    And the scenario should contain "When [action]"
    And the scenario should contain "Then [expected outcome]"
