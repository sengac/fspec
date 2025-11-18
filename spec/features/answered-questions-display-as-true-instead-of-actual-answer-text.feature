@done
@validation
@example-mapping
@cli
@critical
@BUG-090
Feature: Answered questions display as 'true' instead of actual answer text

  """
  Architecture notes:
  - Fix is in answer-question command (src/commands/answer-question.ts)
  - Also affects generate-scenarios command (feature file comment generation)
  - Question interface has answered: boolean field, needs answer?: string field
  - Feature file generator must use question.answer, not question.answered
  - Data integrity: existing work units may have answered: true without answer text
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System must store answer text in question.answer field, not just answered: true boolean
  #   2. Feature file comments must display actual answer text, not boolean value
  #   3. Work unit output must show complete answer text when displaying answered questions
  #
  # EXAMPLES:
  #   1. Answer question with 'Cache for 24 hours', feature file shows 'A: Cache for 24 hours' not 'A: true'
  #   2. Answer multiline question, all text preserved in work unit JSON and feature file comments
  #   3. Question answered before BUG-090 fix shows 'A: true', after fix shows actual answer text
  #
  # ========================================

  Background: User Story
    As a developer answering Example Mapping questions
    I want to preserve answer text instead of boolean true
    So that decision context is retained for documentation and knowledge sharing

  Scenario: Store answer text in question.answer field
    Given a work unit with a question "@human: When should data be cached?"
    When I answer the question with "Cache for 24 hours with file-based persistence"
    Then the question.answered field should be true
    And the question.answer field should contain "Cache for 24 hours with file-based persistence"
    And the question.answer should NOT be the boolean true

  Scenario: Display answer text in feature file comments
    Given a work unit with an answered question
    And the answer text is "Data should be cached immediately on first access"
    When I generate scenarios
    Then the feature file should contain "A: Data should be cached immediately on first access"
    And the feature file should NOT contain "A: true"

  Scenario: Preserve multiline answer text
    Given a work unit with a question
    When I answer with multiline text:
      """
      Caching strategy:
      1. Cache on first access
      2. Use file-based persistence
      3. TTL of 24 hours
      """
    Then the answer text should be stored exactly as provided
    And feature file comments should show all lines of the answer