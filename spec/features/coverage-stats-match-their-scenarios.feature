@BUG-639
@done
@validator
@coverage
@validation
@bug-fix
@rust
@BUG-608
Feature: Coverage stats match their scenarios
  """
  A coverage file's stats block (totalScenarios, coveredScenarios, coveragePercent, testFiles, implFiles, totalLinesCovered) is a deterministic function of its scenarios array. The one function that computes it is the one link-coverage writes stats with; every fspec command that writes a coverage file uses it, and fspec audit's coverage area recomputes it for every coverage file it reads. A difference is the problem coverage stats-mismatch, naming each field with its stored and recomputed value, and fspec audit --fix rewrites only the stats block. A file without a stats block is not reported (show-coverage computes its stats).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A coverage file's stats block is a deterministic function of its scenarios array, computed by the one function link-coverage writes stats with
  #   2. The audit's coverage area reports one coverage stats-mismatch problem per file whose stored stats block differs, naming each field with the stored and the recomputed value; a file without a stats block is not reported
  #   3. fspec audit --fix rewrites the stats block with the recomputed values and changes nothing else in the file
  #   4. Every fspec command that writes a coverage file writes stats with that same function
  #
  # EXAMPLES:
  #   1. 14 scenarios, 11 with test mappings, stats saying 8 and 0: reported as totalScenarios 8 -> 14, coveredScenarios 0 -> 11, coveragePercent 0 -> 79
  #   2. fspec audit --fix --feature <f> rewrites only that file's stats; a second audit reports no stats problem
  #   3. A coverage file link-coverage, unlink-coverage, generate-coverage or add-scenario just wrote passes the check
  #   4. Stats listing a test file no mapping uses any more are reported with testFiles stored and recomputed
  #
  # ========================================
  Background: User Story
    As a developer relying on fspec audit before committing
    I want the audit to report a coverage file whose stats disagree with its scenarios and repair it with fspec audit --fix
    So that show-coverage and every reader of the stats never show counts the mappings do not support

  Scenario: Stats that disagree with the scenarios are reported field by field
    Given a coverage file with 14 scenarios, 11 of them with a test mapping
    And its stats say totalScenarios 8, coveredScenarios 0 and coveragePercent 0
    When fspec audit checks the coverage area
    Then it reports coverage stats-mismatch for the feature
    And the problem names totalScenarios 8 -> 14, coveredScenarios 0 -> 11 and coveragePercent 0 -> 79
    And the problem's repair is fspec audit --fix

  Scenario: Stats that list a test file no mapping uses are reported
    Given a coverage file whose stats list a test file that none of its mappings name
    When fspec audit checks the coverage area
    Then it reports coverage stats-mismatch naming testFiles with the stored and the recomputed list

  Scenario: A coverage file without a stats block is not reported
    Given a coverage file that has scenarios but no stats block
    When fspec audit checks the coverage area
    Then it reports no coverage stats-mismatch

  Scenario: The repair rewrites only the stats block
    Given a coverage file whose stats disagree with its scenarios
    When fspec audit --fix runs for the feature
    Then the stats block holds the recomputed values
    And the scenarios array is unchanged
    And a second audit reports no stats-mismatch

  Scenario: Coverage files fspec writes pass the check
    Given a feature whose coverage file was written by add-scenario, generate-coverage, link-coverage and unlink-coverage in turn
    When fspec audit checks the coverage area after each command
    Then it never reports coverage stats-mismatch

  Scenario: Stats stored as counts make the file unreadable and the repair is fspec audit --fix
    Given a coverage file whose stats store testFiles and implFiles as counts
    When fspec audit checks the coverage area
    Then it reports coverage unreadable-coverage with the exact parse error
    And the problem's repair is fspec audit --fix for the feature
    And after fspec audit --fix the file reads and the audit reports no coverage problem
