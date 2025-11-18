@done
@critical
@validator
@validation
@VAL-005
Feature: Multiple test files mapped to single feature file causes validation errors

  """
  In update-work-unit-status.ts validateTestFilesHaveStepComments() after collecting workUnitTestFiles, check if workUnitTestFiles.size > 1. If true, throw error blocking transition. Error message should tell user to split feature file into multiple smaller features with 1:1 mapping.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When multiple test files are linked to one feature, emit system-reminder suggesting to split the feature file
  #   2. Design intent is 1 feature file = 1 test file for simplicity and maintainability
  #   3. ENFORCE: 1 feature file = 1 test file ONLY
  #   4. If workUnitTestFiles.size > 1, throw error and BLOCK transition
  #
  # EXAMPLES:
  #   1. Coverage file shows 5 test files linked to one feature. System emits reminder: 'MULTIPLE TEST FILES DETECTED... RECOMMENDED: Split this feature file into multiple smaller features.'
  #   2. Current bug: PerplexityResearch.test.ts is validated against 'Navigate between layouts' scenario even though coverage links that scenario to useKeyboardNavigation.test.ts. After fix: PerplexityResearch.test.ts only validated against scenarios it's actually linked to
  #   3. Coverage has 1 test file linked to feature -> validation proceeds normally
  #   4. Coverage has 2 test files linked to feature -> throw error: 'Multiple test files detected. Split feature file.'
  #
  # ========================================

  Background: User Story
    As a AI agent moving work unit through workflow
    I want to enforce 1 feature file = 1 test file
    So that I maintain clean 1:1 mapping and don't allow multiple test files per feature

  Scenario: Allow transition when 1 feature has 1 test file
    Given a work unit has 1 feature file
    And the coverage file links 1 test file to the feature
    When validation runs
    Then validation should proceed normally
    And the transition should succeed

  Scenario: Block transition when 1 feature has multiple test files
    Given a work unit has 1 feature file
    And the coverage file links 2 or more test files to the feature
    When validation runs
    Then an error should be thrown
    And the error should say "Multiple test files detected"
    And the error should say "Split feature file"
    And the transition should be blocked
