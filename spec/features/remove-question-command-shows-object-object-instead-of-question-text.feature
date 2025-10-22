@done
@feature-management
@discovery
@cli
@BUG-020
Feature: remove-question command shows '[object Object]' instead of question text
  """
  Bug in src/commands/remove-question.ts: Success message displays '[object Object]' instead of question text. Root cause: Passing entire question object to console.log instead of extracting the text property. Fix: Access question.question property before display.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Success message must display the actual question text, not '[object Object]'
  #   2. Message format should be 'Removed question: <question text>'
  #
  # EXAMPLES:
  #   1. Remove question 'Should we support OAuth?' displays 'Removed question: Should we support OAuth?'
  #   2. Remove question with special characters '@human: What happens?' displays correctly
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to see the actual question text when removing a question
    So that I can confirm I'm removing the correct question

  Scenario: Remove question displays actual question text
    Given I have a work unit "TEST-001" with a question "Should we support OAuth?"
    When I run "fspec remove-question TEST-001 0"
    Then the success message should display "Removed question: Should we support OAuth?"
    And the message should not contain "[object Object]"

  Scenario: Remove question with special characters displays correctly
    Given I have a work unit "TEST-002" with a question "@human: What happens?"
    When I run "fspec remove-question TEST-002 0"
    Then the success message should display "Removed question: @human: What happens?"
    And the message should not contain "[object Object]"
