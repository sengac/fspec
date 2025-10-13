Feature: Tag Validation Reminders

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  @REMIND-004
  Scenario: User runs 'fspec add-tag-to-feature login.feature @unregistered-tag', reminder shows 'Tag not registered in tags.json'
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-004
  Scenario: User runs 'fspec add-tag-to-feature login.feature @phase1', no unregistered tag reminder but may show missing required tags reminder
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-004
  Scenario: User adds all required tags, no reminder shown
    Given [precondition]
    When [action]
    Then [expected outcome]
