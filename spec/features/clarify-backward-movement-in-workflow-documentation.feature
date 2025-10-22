@done
@cli
@workflow
@documentation
@DOC-003
Feature: Clarify backward movement in workflow documentation

  Background: User Story
    As a developer using fspec workflow
    I want to understand backward state transitions
    So that I can confidently move work backward when fixing mistakes

  Scenario: Move from testing to specifying when tests revealed incomplete acceptance criteria
    Given a work unit is in testing status
    When I run 'fspec update-work-unit-status WORK-001 specifying'
    Then the work unit status should change to specifying

  Scenario: Move from implementing to testing when test cases need refactoring
    Given a work unit is in implementing status
    When I run 'fspec update-work-unit-status WORK-001 testing'
    Then the work unit status should change to testing

  Scenario: Move from validating to implementing when quality checks fail
    Given a work unit is in validating status
    When I run 'fspec update-work-unit-status WORK-001 implementing'
    Then the work unit status should change to implementing

  Scenario: Move from done to implementing when bug discovered in completed feature
    Given a work unit is in done status
    When I run 'fspec update-work-unit-status WORK-001 implementing'
    Then the work unit status should change to implementing

  Scenario: Try to move from backlog to testing - blocked (must go through specifying)
    Given a work unit is in backlog status
    When I run 'fspec update-work-unit-status WORK-001 testing'
    Then the command should fail with an error message explaining that specifying is required first

  Scenario: Move from implementing to specifying when requirements misunderstood
    Given a work unit is in implementing status
    When I run 'fspec update-work-unit-status WORK-001 specifying'
    Then the work unit status should change to specifying
