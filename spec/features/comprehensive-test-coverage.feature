@phase1
@validation
@cli
@unit-test
@critical
Feature: Implement comprehensive test coverage for all commands

  Background: User Story
    As a developer maintaining fspec CLI
    I want comprehensive test coverage for all commands
    So that I can ensure reliability and catch regressions early

  @TEST-001
  Scenario: Test create-feature with all template variations
    Given [precondition]
    When [action]
    Then [expected outcome]

  @TEST-001
  Scenario: Test validation with invalid Gherkin syntax
    Given [precondition]
    When [action]
    Then [expected outcome]

  @TEST-001
  Scenario: Test format preserves content and fixes indentation
    Given [precondition]
    When [action]
    Then [expected outcome]

  @TEST-001
  Scenario: Test file operations handle missing directories
    Given [precondition]
    When [action]
    Then [expected outcome]

  @TEST-001
  Scenario: Test tag registry operations (add, update, delete)
    Given [precondition]
    When [action]
    Then [expected outcome]
