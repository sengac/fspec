@phase1
@work-unit-management
@cli
@system-reminder
Feature: Show Work Unit Reminders

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  @REMIND-006
  Scenario: Show work unit without estimate displays missing estimate reminder
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-006
  Scenario: Show work unit in specifying with no rules/examples displays Example Mapping reminder
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-006
  Scenario: Show work unit that has been in specifying for 25 hours displays long duration reminder
    Given [precondition]
    When [action]
    Then [expected outcome]
