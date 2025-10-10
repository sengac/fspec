@phase4
@phase8
@cli
@querying
@tag-management
@medium
@unit-test
Feature: Get Scenarios by Tag
  """
  Architecture notes:
  - Reads all feature files and parses with @cucumber/gherkin
  - Filters scenarios based on BOTH feature-level tags AND scenario-level tags
  - Scenarios inherit feature tags AND can have their own scenario-level tags
  - Tag matching checks both feature tags and scenario tags
  - Supports multiple tag filtering with AND logic (all tags must match)
  - Returns scenario names, feature file paths, and line numbers
  - Useful for finding scenarios to review, update, or delete
  - Foundation for bulk operations like delete-scenarios --tag

  Critical implementation requirements:
  - MUST parse all feature files in spec/features/
  - MUST match scenarios by BOTH feature-level AND scenario-level tags
  - MUST support filtering by scenario-specific tags (e.g., @smoke, @regression)
  - MUST support multiple --tag flags with AND logic
  - MUST return feature path, scenario name, and line number
  - MUST handle missing spec/features/ directory gracefully
  - MUST skip invalid feature files with warning
  - Exit code 0 for success, 1 for errors

  References:
  - Gherkin parser: @cucumber/gherkin
  - Tag filtering: Checks both feature.tags and scenario.tags
  """

  Background: User Story
    As a developer managing feature specifications
    I want to find all scenarios matching specific tags
    So that I can review, update, or delete them in bulk

  Scenario: Get scenarios from features with single tag
    Given I have 3 feature files tagged with @phase1
    And each feature has 2 scenarios
    When I run `fspec get-scenarios --tag=@phase1`
    Then the command should exit with code 0
    And the output should show 6 scenarios total
    And each scenario should show feature path, scenario name, and line number

  Scenario: Get scenarios with multiple tags (AND logic)
    Given I have feature files with tags @phase1 @critical
    And I have feature files with only @phase1
    And I have feature files with only @critical
    When I run `fspec get-scenarios --tag=@phase1 --tag=@critical`
    Then the output should only show scenarios from features with both tags
    And features with only one of the tags should be excluded

  Scenario: Get scenarios when no features match tags
    Given I have feature files without @deprecated tag
    When I run `fspec get-scenarios --tag=@deprecated`
    Then the command should exit with code 0
    And the output should show "No scenarios found matching tags: @deprecated"

  Scenario: Show scenario details with line numbers
    Given I have a feature file "spec/features/login.feature" tagged @auth
    And the feature has scenario "Successful login" at line 15
    And the feature has scenario "Failed login" at line 25
    When I run `fspec get-scenarios --tag=@auth`
    Then the output should show "login.feature:15 - Successful login"
    And the output should show "login.feature:25 - Failed login"

  Scenario: Handle missing spec/features directory
    Given spec/features/ directory does not exist
    When I run `fspec get-scenarios --tag=@phase1`
    Then the command should exit with code 1
    And the output should show error "spec/features directory not found"

  Scenario: Skip invalid feature files with warning
    Given I have 2 valid feature files tagged @phase1
    And I have 1 invalid feature file with syntax errors
    When I run `fspec get-scenarios --tag=@phase1`
    Then the command should exit with code 0
    And the output should show scenarios from the 2 valid files
    And the output should show warning about the invalid file

  Scenario: Get scenarios from all features when no tag specified
    Given I have 5 feature files with various tags
    When I run `fspec get-scenarios`
    Then the command should exit with code 0
    And the output should show all scenarios from all 5 features
    And the output should show total count of scenarios

  Scenario: Handle features with no scenarios
    Given I have a feature file tagged @phase1 with no scenarios
    And I have a feature file tagged @phase1 with 3 scenarios
    When I run `fspec get-scenarios --tag=@phase1`
    Then the output should only show the 3 scenarios
    And the empty feature should not appear in output

  Scenario: Format output as JSON
    Given I have feature files tagged @phase1 with scenarios
    When I run `fspec get-scenarios --tag=@phase1 --format=json`
    Then the output should be valid JSON
    And each scenario should have feature, name, and line properties
    And the JSON should be parseable by other tools

  Scenario: Group scenarios by feature file
    Given I have feature file "login.feature" with 3 scenarios
    And I have feature file "signup.feature" with 2 scenarios
    When I run `fspec get-scenarios --tag=@auth`
    Then the output should group scenarios by feature file
    And each group should show the feature name as a header
    And scenarios should be listed under their feature

  Scenario: Count scenarios matching tags
    Given I have 10 scenarios across various features
    And 6 of them are in features tagged @critical
    When I run `fspec get-scenarios --tag=@critical`
    Then the output should show "Found 6 scenarios matching tags: @critical"

  Scenario: Filter scenarios by scenario-level tags
    Given I have a feature file with scenario-level tags:
      """
      Feature: User Authentication

        @smoke
        Scenario: Quick login test
          Given I am on the login page
          When I enter credentials
          Then I am logged in

        @regression
        @edge-case
        Scenario: Login with expired session
          Given I have an expired session
          When I attempt to login
          Then I am prompted to re-authenticate

        Scenario: Standard login
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
      """
    When I run `fspec get-scenarios --tag=@smoke`
    Then the command should exit with code 0
    And the output should show only the "Quick login test" scenario
    And the output should not show "Login with expired session"
    And the output should not show "Standard login"

  Scenario: Filter scenarios by multiple scenario-level tags with AND logic
    Given I have a feature file with multiple scenario tags:
      """
      Feature: User Authentication

        @smoke
        @critical
        Scenario: Critical smoke test
          Given a step
          When another step
          Then result

        @smoke
        Scenario: Regular smoke test
          Given a step
          When another step
          Then result

        @regression
        @critical
        Scenario: Critical regression test
          Given a step
          When another step
          Then result
      """
    When I run `fspec get-scenarios --tag=@smoke --tag=@critical`
    Then the output should show only "Critical smoke test"
    And the output should not show "Regular smoke test"
    And the output should not show "Critical regression test"

  Scenario: Match scenarios with inherited feature tags
    Given I have a feature file with feature-level tags:
      """
      @phase1
      @authentication
      Feature: User Login

        Scenario: Basic login
          Given I am on the login page
          When I enter credentials
          Then I am logged in

        @smoke
        Scenario: Quick test
          Given a quick test
          When I run it
          Then it passes
      """
    When I run `fspec get-scenarios --tag=@phase1`
    Then the output should show both scenarios
    And both scenarios inherit the @phase1 tag from the feature

  Scenario: Filter scenarios combining feature tags and scenario tags
    Given I have a feature file with both feature and scenario tags:
      """
      @phase1
      @authentication
      Feature: User Login

        @smoke
        Scenario: Smoke test login
          Given I am on the login page
          When I enter credentials
          Then I am logged in

        @regression
        Scenario: Regression test login
          Given I am on the login page
          When I enter different credentials
          Then I am logged in
      """
    When I run `fspec get-scenarios --tag=@authentication --tag=@smoke`
    Then the output should show only "Smoke test login"
    And the scenario matches both feature tag (@authentication) and scenario tag (@smoke)

  Scenario: Show scenario tags in output
    Given I have a feature file with scenario-level tags:
      """
      Feature: Testing

        @smoke
        @critical
        Scenario: Important test
          Given a step
          When another step
          Then result
      """
    When I run `fspec get-scenarios --tag=@smoke`
    Then the output should show scenario tags in the output
    And the output should display "[@smoke @critical]" next to the scenario name
