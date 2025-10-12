@phase1
@cli
@safety
@example-mapping
@wip
@SAFE-002
Feature: Stable Question Indices for Concurrent Answers
  """
  Architecture notes:
  - Change questions from string array to object array with stable indices
  - Question structure: {text: string, selected: boolean, answer?: string}
  - When question is answered, set selected=true and add answer field
  - Indices never change, preventing race conditions in concurrent operations
  - No file locking needed since each operation modifies only its own index
  - Backward compatible migration for existing work units with string array questions

  Critical implementation requirements:
  - MUST change WorkUnit.questions type from string[] to QuestionItem[]
  - MUST update add-question command to create {text, selected: false} objects
  - MUST update answer-question command to set selected=true instead of removing
  - MUST update show-work-unit to display only unselected questions
  - MUST maintain indices stable even after questions are selected
  - MUST handle backward compatibility with existing string[] questions
  - MUST update TypeScript types in src/types.ts
  """

  Background: User Story
    As a developer using fspec with Claude Code running parallel commands
    I want questions to use stable indices that don't shift when answered
    So that concurrent answer-question commands don't cause race conditions or data loss

  Scenario: Add questions as objects with selected flag
    Given I have a work unit in specifying status
    When I run `fspec add-question WORK-001 "@human: Should we use option A?"`
    Then the question should be added as {text: "@human: Should we use option A?", selected: false}
    And the question array should preserve the object structure

  Scenario: Answer question marks it as selected without removing
    Given I have a work unit with 3 questions at indices 0, 1, 2
    And all questions have selected: false
    When I run `fspec answer-question WORK-001 1 --answer "Use option B" --add-to rule`
    Then question at index 1 should have selected: true
    And question at index 1 should have answer: "Use option B"
    And questions at indices 0 and 2 should still have selected: false
    And all 3 indices should remain in the array (0, 1, 2)

  Scenario: Concurrent answer commands with stable indices
    Given I have a work unit with 3 questions at indices 0, 1, 2
    And all questions have selected: false
    When I run 3 answer-question commands in parallel:
      | index | answer     | add-to |
      | 0     | "Answer 0" | rule   |
      | 1     | "Answer 1" | rule   |
      | 2     | "Answer 2" | rule   |
    Then all 3 questions should have selected: true
    And all 3 answers should be saved in their respective questions
    And all 3 rules should be added to the work unit rules array
    And no data loss should occur

  Scenario: Display only unselected questions
    Given I have a work unit with 5 questions
    And questions at indices 1 and 3 have selected: true
    When I run `fspec show-work-unit WORK-001`
    Then the output should display only questions at indices 0, 2, and 4
    And the displayed indices should match their actual array positions
    And selected questions should not appear in the questions list

  Scenario: Validate unanswered questions before testing phase
    Given I have a work unit in specifying status
    And the work unit has 3 questions
    And question at index 1 has selected: true
    And questions at indices 0 and 2 have selected: false
    When I run `fspec update-work-unit-status WORK-001 testing`
    Then the command should fail with error about unanswered questions
    And the error should list questions at indices 0 and 2

  Scenario: Backward compatibility with string array questions
    Given I have a work unit with questions as string array: ["Q1", "Q2", "Q3"]
    When I run `fspec answer-question WORK-001 0 --answer "A1" --add-to rule`
    Then the questions array should be migrated to object format
    And question 0 should become {text: "Q1", selected: true, answer: "A1"}
    And questions 1 and 2 should become {text: "Q2", selected: false}, {text: "Q3", selected: false}

  Scenario: Answer same question twice is idempotent
    Given I have a work unit with a question at index 0
    And question 0 has selected: false
    When I run `fspec answer-question WORK-001 0 --answer "Answer A" --add-to rule`
    And I run `fspec answer-question WORK-001 0 --answer "Answer B" --add-to rule`
    Then question 0 should have selected: true
    And question 0 should have answer: "Answer B" (last write wins)
    And both rules "Answer A" and "Answer B" should be added
