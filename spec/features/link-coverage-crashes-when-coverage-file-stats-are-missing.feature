@validation
@done
@coverage
@coverage-tracking
@bug-fix
@cli
@BUG-091
Feature: link-coverage crashes when coverage file stats are missing

  """
  Ensure updateStats function checks for existence of stats object before assignment
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. link-coverage must handle missing 'stats' object in coverage files gracefully
  #   2. link-coverage must automatically initialize 'stats' if missing before updating
  #
  # EXAMPLES:
  #   1. User runs link-coverage on a manually created coverage file without stats object
  #
  # ========================================

  Background: User Story
    As a Developer
    I want to link coverage to a feature file
    So that I don't encounter crashes even if the coverage file is incomplete

  Scenario: Link coverage with missing stats object
    Given I have a feature file "test-missing-stats.feature"
    And I have a coverage file "test-missing-stats.feature.coverage" without a stats object
    And I have a test file "test-missing-stats.test.ts"
    When I run "fspec link-coverage test-missing-stats.feature --scenario 'Test Scenario' --test-file test-missing-stats.test.ts --test-lines 1-10"
    Then the exit code should be 0
    And the output should contain "Linked test mapping"
    And the coverage file should contain a valid stats object
