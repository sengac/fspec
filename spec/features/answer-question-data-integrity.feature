@BUG-054
@critical @cli @data-integrity
Feature: Answer Question Data Integrity

  """
  Architecture notes:
  - answer-question command answers questions from Example Mapping
  - When using --add-to rule flag, answer should be added to rules array
  - MUST create proper RuleItem objects, NOT raw strings
  - RuleItem must have structure: {id, text, deleted, createdAt}
  - IDs must use workUnit.nextRuleId++
  - Assumptions are correctly typed as string[] (not a bug)

  Critical bug:
  - Current implementation pushes raw strings: workUnit.rules.push(options.answer)
  - Causes data corruption: show-work-unit displays "[undefined] undefined"
  - Breaks any code that expects RuleItem object structure

  Fix pattern (from add-rule.ts):
    const newRule: RuleItem = {
      id: workUnit.nextRuleId++,
      text: options.answer,
      deleted: false,
      createdAt: new Date().toISOString(),
    };
    workUnit.rules.push(newRule);
  """

  Background: User Story
    As a developer using Example Mapping
    I want answer-question to create proper data structures
    So that work unit data remains valid and doesn't corrupt the JSON

  Scenario: Answer question with --add-to rule creates RuleItem object
    Given a work unit "TEST-001" with a question "Should this be standalone?"
    When I run "fspec answer-question TEST-001 0 --answer 'Yes, standalone script' --add-to rule"
    Then the rules array should contain a RuleItem object
    And the RuleItem should have an id field with a number
    And the RuleItem should have a text field with "Yes, standalone script"
    And the RuleItem should have a deleted field set to false
    And the RuleItem should have a createdAt field with an ISO timestamp
    And show-work-unit should NOT display "[undefined] undefined"

  Scenario: RuleItem ID uses nextRuleId counter
    Given a work unit "TEST-003" with nextRuleId set to 5
    And the work unit has a question "Is validation needed?"
    When I run "fspec answer-question TEST-003 0 --answer 'Yes' --add-to rule"
    Then the new RuleItem should have id 5
    And the work unit nextRuleId should be incremented to 6

  Scenario: Multiple answer-question calls create sequential IDs
    Given a work unit "TEST-004" with two questions
    When I run "fspec answer-question TEST-004 0 --answer 'First answer' --add-to rule"
    And I run "fspec answer-question TEST-004 1 --answer 'Second answer' --add-to rule"
    Then the first RuleItem should have id 0
    And the second RuleItem should have id 1
    And both RuleItems should have proper object structure

  Scenario: Existing corrupt data does not affect new additions
    Given a work unit "TEST-005" with corrupt string data in rules array
    When I run "fspec answer-question TEST-005 0 --answer 'New answer' --add-to rule"
    Then the new entry should be a proper RuleItem object
    And the new RuleItem should have all required fields
