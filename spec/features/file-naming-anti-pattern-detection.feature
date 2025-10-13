Feature: File Naming Anti-Pattern Detection

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  @REMIND-003
  Scenario: User runs 'fspec create-feature implement-authentication', reminder shows '❌ WRONG: implement-authentication → ✅ CORRECT: user-authentication'
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-003
  Scenario: User runs 'fspec create-feature AUTH-001', reminder shows work unit IDs are not capability names
    Given [precondition]
    When [action]
    Then [expected outcome]

  @REMIND-003
  Scenario: User runs 'fspec create-feature user-authentication', no reminder (correct naming)
    Given [precondition]
    When [action]
    Then [expected outcome]
