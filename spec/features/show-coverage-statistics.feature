@reporting
@done
@statistics
@read
@coverage-tracking
@cli
@phase2
Feature: Show Coverage Statistics

  Background: User Story
    As a developer tracking test coverage
    I want to view coverage statistics for feature files
    So that I can identify gaps in test coverage and track testing progress

  Scenario: User runs 'fspec show-coverage user-login.feature', displays markdown report with 80% coverage (4/5 scenarios), full breakdown with symbols, and Coverage Gaps section
    Given I am User
    When I runs 'fspec show-coverage user-login.feature', displays markdown report with 80% coverage (4/5 scenarios), full breakdown with symbols, and Coverage Gaps section
    Then the operation should succeed

  Scenario: User runs 'fspec show-coverage user-login.feature --format=json', outputs JSON with scenarios array, stats object, three-tier coverage status
    Given I am User
    When I runs 'fspec show-coverage user-login.feature --format=json', outputs JSON with scenarios array, stats object, three-tier coverage status
    Then the operation should succeed

  Scenario: User runs 'fspec show-coverage' (no args), displays project summary with aggregated stats + per-feature breakdown for all .coverage files
    Given I am User
    When I runs 'fspec show-coverage' (no args), displays project summary with aggregated stats + per-feature breakdown for all .coverage files
    Then the operation should succeed

  Scenario: Scenario has test mapping with empty implMappings array, displayed as '⚠️ PARTIALLY COVERED' with warning icon
    Given Scenario has test mapping with empty implMappings array, displayed as '⚠️ PARTIALLY COVERED' with warning icon
    When I perform the operation
    Then the result should be as expected

  Scenario: Test file path doesn't exist on disk, display '⚠️ File not found: src/__tests__/deleted.test.ts' warning but still show coverage
    Given I am Test file path doesn't exist on disk, display '⚠️ File not found: src/__tests__/deleted.test.ts' warning but still
    When I show coverage
    Then the operation should succeed

  Scenario: User runs 'fspec show-coverage missing.feature', .coverage file doesn't exist, error with exit code 1 and suggestion message
    Given I am User
    When I runs 'fspec show-coverage missing.feature', .coverage file doesn't exist, error with exit code 1 and suggestion message
    Then the operation should succeed

  Scenario: .coverage file has invalid JSON, command fails with parse error showing line number and suggestion to recreate
    Given .coverage file has invalid JSON, command fails with parse error showing line number and suggestion to recreate
    When I perform the operation
    Then the result should be as expected
