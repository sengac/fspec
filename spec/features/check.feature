@phase1
@cli
@validation
@utility
@high
@integration-test
Feature: Run All Validations
  """
  Architecture notes:
  - Runs all validation checks in one command
  - Validates Gherkin syntax in all feature files
  - Validates tags against TAGS.md registry
  - Checks formatting with Prettier
  - Combines validate, validate-tags, and format --check
  - Returns aggregate status of all checks

  Critical implementation requirements:
  - MUST run Gherkin syntax validation (validate)
  - MUST run tag validation (validate-tags)
  - MUST run formatting check (format --check)
  - MUST report all errors from all checks
  - MUST exit with code 0 only if ALL checks pass
  - MUST exit with code 1 if ANY check fails
  - Output should show status of each check
  - Should be comprehensive pre-commit validation
  """

  Background: User Story
    As a developer preparing to commit changes
    I want to run all validation checks at once
    So that I can ensure my feature files are correct

  Scenario: All validation checks pass
    Given I have 3 valid feature files with registered tags
    And all files are properly formatted
    When I run `fspec check`
    Then the command should exit with code 0
    And the output should show "Gherkin syntax: PASS"
    And the output should show "Tag validation: PASS"
    And the output should show "Formatting: PASS"
    And the output should show "All checks passed"

  Scenario: Gherkin syntax validation fails
    Given I have a feature file with invalid Gherkin syntax
    When I run `fspec check`
    Then the command should exit with code 1
    And the output should show "Gherkin syntax: FAIL"
    And the output should show the Gherkin error details

  Scenario: Tag validation fails
    Given I have a feature file with unregistered tag "@unknown-tag"
    When I run `fspec check`
    Then the command should exit with code 1
    And the output should show "Tag validation: FAIL"
    And the output should show the unregistered tag "@unknown-tag"

  Scenario: Formatting check fails
    Given I have a feature file with incorrect formatting
    When I run `fspec check`
    Then the command should exit with code 1
    And the output should show "Formatting: FAIL"
    And the output should show which files need formatting

  Scenario: Multiple validation failures
    Given I have a feature file with invalid Gherkin syntax
    And I have a feature file with unregistered tag "@bad-tag"
    And I have a feature file with incorrect formatting
    When I run `fspec check`
    Then the command should exit with code 1
    And the output should show "Gherkin syntax: FAIL"
    And the output should show "Tag validation: FAIL"
    And the output should show "Formatting: FAIL"
    And the output should list all errors

  Scenario: No feature files exist
    Given I have no feature files
    When I run `fspec check`
    Then the command should exit with code 0
    And the output should show "No feature files found"

  Scenario: Check reports file counts
    Given I have 10 valid feature files
    When I run `fspec check`
    Then the command should exit with code 0
    And the output should show "Checked 10 feature file(s)"

  Scenario: Check validates all feature files
    Given I have 5 feature files
    And 1 file has invalid Gherkin syntax
    When I run `fspec check`
    Then the command should exit with code 1
    And the output should show which file has invalid syntax
    And the output should show the line number of the error

  Scenario: Check with verbose output
    Given I have 3 valid feature files
    When I run `fspec check --verbose`
    Then the command should exit with code 0
    And the output should list each file validated
    And the output should show detailed check results

  Scenario: Check runs quickly on large repositories
    Given I have 100 valid feature files
    When I run `fspec check`
    Then the command should exit with code 0
    And the command should complete within a reasonable time

  Scenario: Check output is CI-friendly
    Given I have a feature file with validation errors
    When I run `fspec check` in a CI environment
    Then the command should exit with code 1
    And the output should be machine-readable
    And error messages should include file paths and line numbers
