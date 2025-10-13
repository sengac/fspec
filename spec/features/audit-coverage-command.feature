@done
@coverage-tracking
@cli
@phase2
@COV-005
Feature: Audit Coverage Command

  Background: User Story
    As a developer maintaining coverage tracking
    I want to audit and update line number mappings when code changes
    So that I keep coverage data accurate as code evolves

  Scenario: Audit coverage with all files present
    Given I have a coverage file "user-login.feature.coverage"
    And all test files referenced in coverage exist
    And all implementation files referenced in coverage exist
    When I run `fspec audit-coverage user-login`
    Then the command should display "✅ All files found (3/3)"
    And the output should show "All mappings valid"
    And the command should exit with code 0

  Scenario: Audit detects missing test file
    Given I have a coverage file with mapping to "src/__tests__/deleted.test.ts"
    And the test file "src/__tests__/deleted.test.ts" does not exist
    When I run `fspec audit-coverage user-login`
    Then the output should display "❌ Test file not found: src/__tests__/deleted.test.ts"
    And the output should show recommendation "Remove this mapping or restore the deleted file"
    And the command should exit with code 1

  Scenario: Audit detects missing implementation file
    Given I have a coverage file with mapping to "src/auth/deleted.ts"
    And the implementation file "src/auth/deleted.ts" does not exist
    When I run `fspec audit-coverage user-login`
    Then the output should display "❌ Implementation file not found: src/auth/deleted.ts"
    And the output should show actionable recommendation
    And the command should exit with code 1
