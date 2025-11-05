@done
@medium
@cli
@feature-management
@BUG-068
Feature: fspec add-step appends instead of replacing prefill placeholders
  """
  Architecture notes:
  - Uses @cucumber/gherkin parser to parse and modify feature files via AST
  - Parser infrastructure already exists in feature-formatter.ts
  - add-step command currently appends steps after existing steps
  - Need to detect placeholder steps (bracket-wrapped keywords) before appending

  Critical implementation requirements:
  - MUST parse feature file AST to detect placeholder steps
  - MUST replace placeholder step text with actual step text (not append)
  - MUST maintain backward compatibility (append if no placeholder exists)
  - Placeholder mapping: precondition → Given, action → When, expected outcome → Then
  - MUST preserve formatting and indentation after replacement

  Dependencies:
  - @cucumber/gherkin (already in use)
  - feature-formatter.ts (AST parsing/formatting utilities)
  """

  Background: User Story
    As a developer using fspec CLI
    I want to add steps to scenarios with placeholders
    So that new steps replace placeholders instead of creating duplicates

  Scenario: Replace Given placeholder with actual step
    Given a feature file with scenario containing a Given step with placeholder text
    When I run fspec add-step with given keyword and text "work unit moves to testing state"
    Then the placeholder step should be replaced with "Given work unit moves to testing state"
    And there should be no duplicate Given steps

  Scenario: Replace When placeholder with actual step
    Given a feature file with scenario containing a When step with placeholder text
    When I run fspec add-step with when keyword and text "user clicks submit"
    Then the placeholder step should be replaced with "When user clicks submit"
    And there should be no duplicate When steps

  Scenario: Replace Then placeholder with actual step
    Given a feature file with scenario containing a Then step with placeholder text
    When I run fspec add-step with then keyword and text "form should be submitted"
    Then the placeholder step should be replaced with "Then form should be submitted"
    And there should be no duplicate Then steps

  Scenario: Append step when no placeholder exists
    Given a feature file with scenario containing "Given actual existing step"
    When I run fspec add-step with given keyword and text "another step"
    Then the new step "Given another step" should be appended after existing step
    And both Given steps should exist in the scenario

  Scenario: Replace multiple placeholders in sequence
    Given a feature file with scenario containing placeholders for Given, When, and Then
    When I run fspec add-step for given with text "initial state"
    And I run fspec add-step for when with text "action occurs"
    And I run fspec add-step for then with text "expected result"
    Then all placeholders should be replaced with actual steps
    And the scenario should have no remaining placeholders
