@validation
@done
@coverage
@coverage-tracking
@refactoring
@cli
@REFAC-005
Feature: Refactor link-coverage command to reduce file size and complexity

  """
  Split link-coverage.ts into core logic, validation, and stats update modules
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. link-coverage.ts must be split into smaller, focused modules
  #   2. File size must be reduced below 300 lines
  #   3. Functionality must remain identical to the original implementation
  #
  # EXAMPLES:
  #   1. Developer runs link-coverage after refactoring and it works exactly as before
  #   2. Developer checks file sizes and confirms link-coverage.ts is < 300 lines
  #
  # ========================================

  Background: User Story
    As a Developer
    I want to refactor the link-coverage command
    So that the code is maintainable and complies with project standards

  Scenario: Verify link-coverage functionality after refactoring
    Given the link-coverage command has been refactored
    When I run "fspec link-coverage test.feature --scenario 'Test Scenario' --test-file test.ts --test-lines 1-10"
    Then the exit code should be 0
    And the output should contain "Linked test mapping"
    And the coverage file should be updated correctly

  Scenario: Verify file size reduction
    Given the link-coverage command has been refactored
    When I check the file size of "src/commands/link-coverage.ts"
    Then the file size should be less than 300 lines
