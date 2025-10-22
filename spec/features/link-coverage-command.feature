@done
@modification
@coverage-tracking
@cli
Feature: Link Coverage Command

  Background: User Story
    As a developer tracking test coverage
    I want to link scenarios to test files and implementation files
    So that I can track which code covers which acceptance criteria

  Scenario: User runs 'fspec link-coverage user-login --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --test-lines 45-62', creates test mapping, displays success and helpful message
    Given I am User
    When I runs 'fspec link-coverage user-login --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --test-lines 45-62', creates test mapping, displays success and helpful message
    Then the operation should succeed

  Scenario: User runs 'fspec link-coverage user-login --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10,11,12', adds impl mapping to existing test
    Given I am User
    When I runs 'fspec link-coverage user-login --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10,11,12', adds impl mapping to existing test
    Then the operation should succeed

  Scenario: User runs command with both test and impl flags at once, creates test mapping with impl mapping in one operation
    Given I am User
    When I runs command with both test and impl flags at once, creates test mapping with impl mapping in one operation
    Then the operation should succeed

  Scenario: Test file doesn't exist, validation fails with error and suggestion to use --skip-validation
    Given I have an invalid condition
    When I execute Test file doesn't exist, validation
    Then it should fails with error and suggestion to use --skip-validation

  Scenario: Adding impl file that already exists in test mapping updates line numbers instead of creating duplicate entry
    Given I am Adding impl file that already exists in test mapping
    When I updates line numbers instead of creating duplicate entry
    Then the operation should succeed

  Scenario: Adding test mapping with same test file but different lines appends as second mapping
    Given I have a scenario with an existing test mapping
    When I add another test mapping with the same test file but different line range
    Then both test mappings should exist in the coverage file
