@done
@critical
@cli
@validation
@example-mapping
@BUG-060
Feature: generate-scenarios counts deleted questions as unanswered
  """
  Bug in generate-scenarios.ts:308-311 where validation filters questions by \!questionItem.selected but doesn't check \!questionItem.deleted. Uses QuestionItem type with deleted and selected boolean fields.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Validation must only count questions that are NOT deleted AND NOT answered (selected=false)
  #   2. Deleted questions (deleted=true) must be ignored in all validation checks
  #   3. Answered questions have selected=true (answer field populated)
  #   4. Validation must filter out deleted questions (questions with deleted=true property) before counting unanswered questions
  #   5. This bug exists in BOTH generate-scenarios.ts AND update-work-unit-status.ts
  #
  # EXAMPLES:
  #   1. Work unit has 3 deleted questions (deleted=true, selected=false) and 5 answered questions (deleted=false, selected=true). Current code counts 3 unanswered, blocks generation. Correct behavior: 0 unanswered, allows generation.
  #   2. Work unit has 2 unanswered questions (deleted=false, selected=false). Current code correctly counts 2 unanswered, blocks generation. Behavior is correct.
  #   3. Work unit has no questions. Current code correctly counts 0 unanswered, allows generation. Behavior is correct.
  #   4. User deletes 3 questions, validation counts them as unanswered, blocks state transition
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to generate scenarios from Example Mapping
    So that I can proceed after answering all questions, without being blocked by deleted questions

  Scenario: Deleted questions should not be counted as unanswered
    Given a work unit has 3 deleted questions with deleted=true and selected=false
    And the work unit has 5 answered questions with deleted=false and selected=true
    When I run "fspec generate-scenarios <work-unit-id>"
    Then the validation should count 0 unanswered questions
    And the command should succeed and generate scenarios
    And the command should not throw "unanswered questions found" error

  Scenario: Unanswered non-deleted questions should block generation
    Given a work unit has 2 unanswered questions with deleted=false and selected=false
    When I run "fspec generate-scenarios <work-unit-id>"
    Then the validation should count 2 unanswered questions
    And the command should fail with "2 unanswered questions found" error
    And the command should not generate scenarios

  Scenario: Work unit with no questions should allow generation
    Given a work unit has no questions
    When I run "fspec generate-scenarios <work-unit-id>"
    Then the validation should count 0 unanswered questions
    And the command should succeed and generate scenarios

  Scenario: Deleted questions should not block status transition in update-work-unit-status
    Given a work unit in "specifying" status has 3 deleted questions with deleted=true and selected=false
    And the work unit has 5 answered questions with deleted=false and selected=true
    When I run "fspec update-work-unit-status <work-unit-id> testing"
    Then the validation should count 0 unanswered questions
    And the command should succeed and update status to "testing"
    And the command should not throw "unanswered questions" error

  Scenario: Unanswered non-deleted questions should block status transition
    Given a work unit in "specifying" status has 2 unanswered questions with deleted=false and selected=false
    When I run "fspec update-work-unit-status <work-unit-id> testing"
    Then the validation should count 2 unanswered questions
    And the command should fail with "Unanswered questions prevent state transition" error
    And the status should remain "specifying"
