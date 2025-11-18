@critical
@cli
@event-storm
@example-mapping
@validation
@BUG-088
Feature: Malformed question text when transforming Event Storm hotspots to Example Mapping

  """
  Architecture notes:
  - Fix is in generate-example-mapping-from-event-storm command
  - Hotspot transformation function must preserve concern text as-is
  - No template wrapping like "What should X be?" pattern
  - Simple transformation: prepend '@human: ' + ensure trailing '?'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When hotspot concern is already a question, use concern text as-is with '@human:' prefix
  #   2. Preserve original capitalization of concern text (don't lowercase)
  #   3. Ensure question ends with '?' if concern lacks one
  #
  # EXAMPLES:
  #   1. Concern 'When should playlists be saved?' → '@human: When should playlists be saved?'
  #   2. Concern 'How long should metadata be cached' → '@human: How long should metadata be cached?'
  #   3. Concern 'Should drag-and-drop support multi-select? How to handle edge cases?' → '@human: Should drag-and-drop support multi-select? How to handle edge cases?'
  #
  # ========================================

  Background: User Story
    As a developer transforming Event Storm to Example Mapping
    I want to generate clear, grammatically correct questions from hotspot concerns
    So that questions are readable and usable without manual editing

  Scenario: Transform question-format concern without modification
    Given a hotspot with concern "When should playlists be saved?"
    When I transform Event Storm to Example Mapping
    Then the generated question should be "@human: When should playlists be saved?"
    And the question should not contain "What should"
    And the question should not contain " be?"

  Scenario: Add question mark if concern lacks one
    Given a hotspot with concern "How long should metadata be cached"
    When I transform Event Storm to Example Mapping
    Then the generated question should be "@human: How long should metadata be cached?"
    And the question should preserve the original text
    And the question should end with "?"

  Scenario: Preserve multiple sentences in concern
    Given a hotspot with concern "Should drag-and-drop support multi-select? How to handle edge cases?"
    When I transform Event Storm to Example Mapping
    Then the generated question should be "@human: Should drag-and-drop support multi-select? How to handle edge cases?"
    And both sentences should be preserved
    And capitalization should be preserved
