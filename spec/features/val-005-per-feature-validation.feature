@BUG-093
Feature: VAL-005 1:1 validation checks across entire work unit instead of per-feature

  """
  Fix is in src/commands/update-work-unit-status.ts lines 885-952. The bug aggregates test files across all features into a single Set, then checks if Set.size > 1. Fix moves the 1:1 check inside the feature loop with a per-feature Set.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. VAL-005 must check 1:1 mapping per individual feature file, not across all features
  #   2. Each feature file must have exactly one test file linked to it
  #   3. A feature file with zero test files should fail validation
  #   4. A feature file with more than one test file should fail validation
  #
  # EXAMPLES:
  #   1. Work unit with 1 feature + 1 test file = PASS
  #   2. Work unit with 1 feature + 2 test files = FAIL (multiple test files for single feature)
  #   3. Work unit with 3 features + 3 test files (1 each) = PASS (currently fails - THIS IS THE BUG)
  #   4. Work unit with 3 features where one has 2 test files = FAIL
  #   5. Work unit with 3 features where one has 0 test files = FAIL
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to have VAL-005 validate 1:1 feature-to-test mapping per individual feature file
    So that work units with multiple features can pass validation when each feature has exactly one test file

  Scenario: Single feature with single test file passes validation
    Given a work unit with 1 feature file linked to 1 test file
    When VAL-005 validation runs during status transition to implementing
    Then the validation passes


  Scenario: Single feature with multiple test files fails validation
    Given a work unit with 1 feature file linked to 2 test files
    When VAL-005 validation runs during status transition to implementing
    Then the validation fails with error about multiple test files for single feature


  Scenario: Multiple features each with single test file passes validation
    Given a work unit with 3 feature files each linked to exactly 1 test file
    When VAL-005 validation runs during status transition to implementing
    Then the validation passes because each feature has exactly one test file


  Scenario: Multiple features where one has multiple test files fails validation
    Given a work unit with 3 feature files where one feature has 2 test files
    When VAL-005 validation runs during status transition to implementing
    Then the validation fails identifying the specific feature with multiple test files


  Scenario: Multiple features where one has no test files fails validation
    Given a work unit with 3 feature files where one feature has 0 test files
    When VAL-005 validation runs during status transition to implementing
    Then the validation fails identifying the specific feature with no test files

