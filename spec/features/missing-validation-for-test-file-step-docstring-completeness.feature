@critical
@cli
@validation
@acdd
@BUG-061
Feature: Missing validation for test file @step docstring completeness

  """
  CRITICAL FIX: Must use .feature.coverage files to find test files (readAllCoverageFiles, extractTestFiles from coverage-reader). NEVER hard-code test file patterns - system is language-agnostic. Coverage files contain testMappings array with actual test file paths used during linking.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Step validation logic exists in src/utils/step-validation.ts with validateSteps() and formatValidationError() functions
  #   2. Step validation is currently ONLY used in link-coverage command (src/commands/link-coverage.ts), NOT in update-work-unit-status
  #   3. Test files MUST have Given, When, AND Then @step docstrings when moving work unit from testing to implementing
  #   4. Test files MUST have Given, When, AND Then @step docstrings when moving work unit to validating state
  #   5. Tasks (type=task) are exempt from step validation since they don't have tests
  #
  # EXAMPLES:
  #   1. Work unit BUG-060 moves from testing to implementing. Test file has complete Given/When/Then @step comments. Currently allowed (correct behavior, should continue working after fix).
  #   2. Work unit STORY-001 moves from testing to implementing. Test file exists but has NO @step docstrings. Currently ALLOWED (bug), should be BLOCKED with system-reminder showing missing steps.
  #   3. Work unit moves to validating. Test file has Given and When steps but missing Then step. Currently ALLOWED (bug), should be BLOCKED showing which steps are missing.
  #
  # ========================================

  Background: User Story
    As a AI agent following ACDD
    I want to ensure test files have complete Given/When/Then step docstrings before implementation
    So that test-to-scenario traceability is maintained and scenarios are properly tested

  Scenario: Allow transition to implementing when test file has complete step docstrings
    Given a work unit with type "story" is in "testing" status
    And the work unit has a linked feature file with scenarios tagged with the work unit ID
    And the test file for the work unit contains complete Given, When, and Then @step docstrings
    When I run "fspec update-work-unit-status <work-unit-id> implementing"
    Then the command should succeed
    And the work unit status should be updated to "implementing"
    And no validation error should be displayed

  Scenario: Block transition to implementing when test file has no step docstrings
    Given a work unit with type "story" is in "testing" status
    And the work unit has a linked feature file with scenarios tagged with the work unit ID
    And the test file for the work unit exists but contains NO @step docstrings
    When I run "fspec update-work-unit-status <work-unit-id> implementing"
    Then the command should fail with a validation error
    And the error should indicate missing step docstrings
    And the error should show which steps are missing
    And the error should be wrapped in a system-reminder tag
    And the work unit status should remain "testing"

  Scenario: Block transition to validating when test file has incomplete step docstrings
    Given a work unit with type "story" is in "implementing" status
    And the work unit has a linked feature file with scenarios tagged with the work unit ID
    And the test file for the work unit contains Given and When @step docstrings
    But the test file is missing Then @step docstrings
    When I run "fspec update-work-unit-status <work-unit-id> validating"
    Then the command should fail with a validation error
    And the error should indicate which steps are missing
    And the error should specifically mention "Then" steps are missing
    And the work unit status should remain "implementing"

  Scenario: Exempt task work units from step validation
    Given a work unit with type "task" is in "specifying" status
    And the work unit has a linked feature file
    And no test file exists for the work unit
    When I run "fspec update-work-unit-status <work-unit-id> implementing"
    Then the command should succeed
    And the work unit status should be updated to "implementing"
    And no step validation should occur
