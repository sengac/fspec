@workflow
@done
@coverage-tracking
@cli
@COV-006
Feature: ACDD Workflow Integration

  Background: User Story
    As a developer using fspec with ACDD workflow
    I want to ensure work units cannot be marked done when coverage is incomplete
    So that I maintain quality standards and prevent incomplete work from being considered complete

  Scenario: Allow status update to done when all scenarios are covered
    Given I have a work unit "AUTH-001" linked to "user-login.feature"
    And the coverage file shows all 5 scenarios have testMappings
    When I run `fspec update-work-unit-status AUTH-001 done`
    Then the command should succeed with exit code 0
    And the work unit status should be updated to "done"

  Scenario: Block status update to done when scenarios are uncovered
    Given I have a work unit "AUTH-001" linked to "user-login.feature"
    And the coverage file shows 2 out of 5 scenarios have empty testMappings
    When I run `fspec update-work-unit-status AUTH-001 done`
    Then the command should fail with exit code 1
    And the output should display "Cannot mark work unit done: 2 scenarios uncovered"
    And the output should show a system-reminder with uncovered scenario names
    And the work unit status should remain "validating"

  Scenario: Allow status update when coverage file doesn't exist
    Given I have a work unit "AUTH-001" linked to "user-login.feature"
    And no coverage file exists for "user-login.feature"
    When I run `fspec update-work-unit-status AUTH-001 done`
    Then the command should succeed with exit code 0
    And the output should display a warning about missing coverage file
    And the work unit status should be updated to "done"
