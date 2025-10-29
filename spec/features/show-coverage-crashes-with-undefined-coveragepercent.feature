@done
@cli
@validation
@coverage
@bug-fix
@backward-compatibility
@BUG-049
Feature: show-coverage crashes with undefined coveragePercent
  """
  show-coverage.ts reads coverage files expecting stats object but legacy files lack it. Fix: add calculateStats() helper function that computes stats from scenarios array when stats is undefined. Ensures backward compatibility with older coverage file formats.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. show-coverage must handle coverage files missing stats object gracefully without crashing
  #   2. When stats object is missing, show-coverage should calculate stats from scenarios array before displaying coverage
  #   3. Calculated stats must match CoverageStats interface: totalScenarios, coveredScenarios, coveragePercent, testFiles, implFiles, totalLinesCovered
  #
  # EXAMPLES:
  #   1. Given coverage file with 4 scenarios (all with testMappings), when stats missing, show-coverage calculates coveragePercent as 100%
  #   2. Given coverage file with 2 scenarios (1 with testMappings, 1 without), when stats missing, show-coverage calculates coveragePercent as 50%
  #   3. Given coverage file with scenarios array but no stats, show-coverage extracts unique test files and impl files for stats.testFiles and stats.implFiles arrays
  #
  # ========================================
  Background: User Story
    As a developer using fspec show-coverage
    I want to view coverage statistics for features with legacy coverage files missing stats object
    So that the command works reliably without crashing on older coverage file formats

  Scenario: Calculate 100% coverage when all scenarios have test mappings
    Given a coverage file exists with 4 scenarios
    And all 4 scenarios have testMappings
    And the coverage file is missing the stats object
    When I run "fspec show-coverage <feature-name>"
    Then the command should not crash
    And the output should show coveragePercent as 100%
    And the output should show "4/4 scenarios"

  Scenario: Calculate 50% coverage when half the scenarios lack test mappings
    Given a coverage file exists with 2 scenarios
    And 1 scenario has testMappings
    And 1 scenario has no testMappings
    And the coverage file is missing the stats object
    When I run "fspec show-coverage <feature-name>"
    Then the command should not crash
    And the output should show coveragePercent as 50%
    And the output should show "1/2 scenarios"

  Scenario: Extract unique test files and impl files when stats missing
    Given a coverage file exists with multiple scenarios
    And the scenarios reference test files "test1.ts" and "test2.ts"
    And the scenarios reference impl files "impl1.ts" and "impl2.ts"
    And the coverage file is missing the stats object
    When I run "fspec show-coverage <feature-name>"
    Then the command should not crash
    And the calculated stats should include testFiles array with ["test1.ts", "test2.ts"]
    And the calculated stats should include implFiles array with ["impl1.ts", "impl2.ts"]
