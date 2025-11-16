@BUG-077
Feature: Coverage validation missing for implementing→validating transition

  """
  Adds coverage validation check at implementing→validating transition in update-work-unit-status.ts (lines ~383-390), reusing existing checkCoverageCompleteness function from lines 410-418. Validation runs when newStatus === 'validating' to ensure implementation files are linked in coverage before entering validation phase.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Step validation must check that validateSteps() actually validates @step comments exist in test file, not just that coverage file exists
  #   2. validateTestStepDocstrings() currently passes if coverage file exists with test mappings, but doesn't verify @step comments
  #   3. Coverage validation (checkCoverageCompleteness) should run when moving from implementing to validating, not just done
  #
  # EXAMPLES:
  #   1. EXMAP-006 moved from testing to implementing to validating without any @step comments in test file, validation should have blocked this
  #   2. EXMAP-006 moved from implementing to validating without any implementation files linked in coverage, should have been blocked
  #
  # ========================================

  Background: User Story
    As a AI agent following ACDD workflow
    I want to move work unit from implementing to validating
    So that system validates implementation exists before allowing validation phase

  Scenario: Block implementing→validating when implementation coverage is missing
    Given I have a work unit "TEST-001" in implementing status
    And the work unit has test files linked in coverage
    And the work unit has NO implementation files linked in coverage
    When I run "fspec update-work-unit-status TEST-001 validating"
    Then the command should fail with error code 1
    And the error message should contain "implementation coverage is incomplete"
    And the error message should suggest "fspec link-coverage" command
    And the work unit status should remain "implementing"
