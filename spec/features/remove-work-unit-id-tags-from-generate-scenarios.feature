@done
@validation
@cli
@phase1
@BUG-005
Feature: Remove work unit ID tags from generate-scenarios

  Background: User Story
    As a developer maintaining fspec
    I want to verify that generate-scenarios adds work unit ID tags only at feature level
    So that I ensure the two-tier linking system (feature-level tags + coverage files) works correctly

  Scenario: Generate scenarios adds work unit ID as feature-level tag only
    Given I have a work unit with ID "TEST-001" in specifying status
    And the work unit has example mapping data (rules, examples, questions answered)
    When I run `fspec generate-scenarios TEST-001`
    Then a feature file should be created with @TEST-001 as a feature-level tag
    And none of the generated scenarios should have @TEST-001 as a scenario-level tag

  Scenario: Verify check.ts uses validateTags function
    Given I have the source file "src/commands/check.ts"
    When I inspect the validation logic
    Then it should import and use the validateTags() function
    And it should not contain inline tag validation logic

  Scenario: Script moves scenario-level work unit ID tags to feature level
    Given I have a feature file with scenario-level work unit ID tags
    And the scenario has @COV-001 tag
    When I run the migration script
    Then the @COV-001 tag should be moved to feature-level
    And the scenario should no longer have @COV-001 tag

  Scenario: Validation rejects scenario-level work unit ID tags
    Given I have enabled the work unit ID tag validation rule
    And I have a feature file with scenario-level work unit ID tag @AUTH-001
    When I run `fspec validate-tags`
    Then the validation should fail
    And the error message should indicate scenario-level work unit ID tags are not allowed
    And the error should show which scenario has the invalid tag
