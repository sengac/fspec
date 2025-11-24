@critical
@cli
@example-mapping
@data-integrity
@BUG-092
Feature: Duplicate question IDs from generate-example-mapping-from-event-storm

  """
  Uses stable indices system (Migration 001) with monotonic ID counters. Command generate-example-mapping-from-event-storm transforms Event Storm hotspots to Example Mapping questions. Must use workUnit.nextQuestionId++ pattern (not array.length) to prevent duplicate IDs. Same pattern applies to rules (nextRuleId) and examples (nextExampleId) for consistency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Question IDs must be assigned using workUnit.nextQuestionId counter, not array length
  #   2. nextQuestionId counter must be incremented after each question is added
  #   3. nextQuestionId must be initialized to 0 if undefined for backward compatibility
  #   4. Same pattern must apply to rules and examples for consistency
  #
  # EXAMPLES:
  #   1. When hotspots are transformed to questions, nextQuestionId starts at 0 if undefined
  #   2. When 3 hotspots are converted, questions get IDs 0, 1, 2 and nextQuestionId becomes 3
  #   3. Running generate-example-mapping-from-event-storm twice does not create duplicate IDs
  #   4. After event storm mapping, manual add-question command gets next sequential ID without collision
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to have unique question IDs after generating example mapping from event storm
    So that data integrity is maintained and no duplicate IDs corrupt the work unit

  Scenario: Initialize nextQuestionId when undefined
    Given a work unit with event storm hotspots
    And the work unit has no questions array or nextQuestionId is undefined
    When I run generate-example-mapping-from-event-storm command
    Then nextQuestionId should be initialized to 0
    And questions should be assigned sequential IDs starting from 0

  Scenario: Assign sequential IDs when converting hotspots
    Given a work unit with 3 event storm hotspots
    And the work unit nextQuestionId is 0
    When I run generate-example-mapping-from-event-storm command
    Then 3 questions should be created with IDs 0, 1, 2
    And nextQuestionId should be 3

  Scenario: Prevent duplicate IDs on multiple invocations
    Given a work unit with event storm hotspots
    And I have run generate-example-mapping-from-event-storm once
    And questions already exist with specific IDs
    When I add more hotspots and run generate-example-mapping-from-event-storm again
    Then new questions should get sequential IDs without duplicates
    And all question IDs should be unique

  Scenario: Integrate with manual add-question command
    Given a work unit with event storm hotspots
    And I have run generate-example-mapping-from-event-storm
    And questions exist with IDs up to N
    When I run manual add-question command
    Then the new question should get ID N+1
    And there should be no ID collision with existing questions
