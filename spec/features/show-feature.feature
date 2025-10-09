@phase2
@cli
@feature-management
@read-only
@low
@unit-test
Feature: Display Feature File Contents
  """
  Architecture notes:
  - Displays the full contents of a feature file
  - Accepts feature file name or path
  - Outputs to stdout or file
  - Optionally formats output as JSON
  - Read-only operation, no modifications

  Critical implementation requirements:
  - MUST accept feature file name or path
  - MUST support output formats: text (default), json
  - MUST support --output flag to write to file
  - MUST validate feature file exists
  - MUST parse Gherkin to validate syntax
  - Text format outputs file contents as-is
  - JSON format outputs parsed Gherkin structure
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer reviewing feature specifications
    I want to display feature file contents
    So that I can read and review specifications

  Scenario: Show feature file contents in text format
    Given I have a feature file "login.feature" with valid Gherkin
    When I run `fspec show-feature login`
    Then the command should exit with code 0
    And the output should contain the feature file contents
    And the output should be in plain text format

  Scenario: Show feature file by full path
    Given I have a feature file "spec/features/checkout.feature"
    When I run `fspec show-feature spec/features/checkout.feature`
    Then the command should exit with code 0
    And the output should contain the feature file contents

  Scenario: Show feature file in JSON format
    Given I have a feature file "api.feature" with Feature, Background, and 2 Scenarios
    When I run `fspec show-feature api --format=json`
    Then the command should exit with code 0
    And the output should be valid JSON
    And the JSON should contain the feature name
    And the JSON should contain the background section
    And the JSON should contain 2 scenarios

  Scenario: Write feature contents to output file
    Given I have a feature file "payment.feature"
    When I run `fspec show-feature payment --output=feature-copy.txt`
    Then the command should exit with code 0
    And the file "feature-copy.txt" should exist
    And "feature-copy.txt" should contain the feature file contents

  Scenario: Show feature with architecture doc string
    Given I have a feature file "auth.feature" with architecture doc string
    When I run `fspec show-feature auth`
    Then the command should exit with code 0
    And the output should contain the architecture doc string

  Scenario: Show feature with tags
    Given I have a feature file "search.feature" with tags "@api @critical"
    When I run `fspec show-feature search`
    Then the command should exit with code 0
    And the output should contain "@api @critical"

  Scenario: Reject non-existent feature file
    Given I have no feature file named "missing.feature"
    When I run `fspec show-feature missing`
    Then the command should exit with code 1
    And the output should show "Feature file not found"

  Scenario: Accept feature name without .feature extension
    Given I have a feature file "spec/features/dashboard.feature"
    When I run `fspec show-feature dashboard`
    Then the command should exit with code 0
    And the output should contain the dashboard feature contents

  Scenario: Show feature with scenario-level tags
    Given I have a feature file "notifications.feature" with scenarios tagged "@email @sms"
    When I run `fspec show-feature notifications`
    Then the command should exit with code 0
    And the output should contain "@email @sms"

  Scenario: JSON format includes all Gherkin elements
    Given I have a feature file "integration.feature" with tags, doc string, Background, and scenarios
    When I run `fspec show-feature integration --format=json`
    Then the command should exit with code 0
    And the JSON should contain feature tags
    And the JSON should contain the architecture doc string
    And the JSON should contain the Background section
    And the JSON should contain all scenarios
    And the JSON should contain all steps

  Scenario: Write JSON output to file
    Given I have a feature file "reporting.feature"
    When I run `fspec show-feature reporting --format=json --output=report.json`
    Then the command should exit with code 0
    And the file "report.json" should exist
    And "report.json" should contain valid JSON

  Scenario: Validate feature file Gherkin syntax
    Given I have a feature file "orders.feature" with valid Gherkin
    When I run `fspec show-feature orders`
    Then the command should exit with code 0
    And the feature file syntax should be validated

  Scenario: Show error for invalid Gherkin syntax
    Given I have a feature file "broken.feature" with invalid Gherkin syntax
    When I run `fspec show-feature broken`
    Then the command should exit with code 1
    And the output should show "Invalid Gherkin syntax"
