@done
@cli
@validator
@critical
@coverage
@validation
@bug-fix
@COV-055
Feature: Coverage File Synchronization

  """
  Notification strategy: (1) Explicit commands (delete-scenario) always notify, (2) Automatic cleanup (generate-coverage) only notify if stale scenarios found, (3) Validation errors always notify. Use ✓ for success, ℹ for informational, ✗ for errors.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a scenario is deleted from a feature file, the corresponding entry must be removed from the .feature.coverage file
  #   2. The updateCoverageFile() function must detect and remove scenarios that exist in coverage file but not in feature file
  #   3. Coverage validation must check that all scenarios in coverage file still exist in the feature file before reporting uncovered scenarios
  #   4. Coverage statistics (totalScenarios, coveredScenarios) must be recalculated after removing deleted scenarios
  #   5. Use explicit rename command (update-scenario) to preserve mappings. When user runs update-scenario, rename coverage entry and preserve test mappings. Manual delete + create loses mappings (expected behavior).
  #   6. Use conditional notifications: (1) Explicit user commands (delete-scenario) always notify about coverage updates, (2) Automatic cleanup (generate-coverage, updateCoverageFile) only notify if stale scenarios found, (3) Validation errors always notify. Use ✓ for successful actions, ℹ for informational cleanup, ✗ for errors.
  #
  # EXAMPLES:
  #   1. Developer deletes 'Scenario B' from test.feature using delete-scenario command. The .feature.coverage file automatically removes the 'Scenario B' entry and recalculates stats.
  #   2. Developer deletes multiple scenarios tagged @deprecated using delete-scenarios-by-tag. All deleted scenarios are removed from coverage files across multiple feature files.
  #   3. Developer runs update-work-unit-status to 'validating'. Coverage checker detects that coverage file has 'Scenario C' but feature file doesn't. System suggests running generate-coverage to sync.
  #   4. Developer runs generate-coverage on a feature that has scenarios A and B in feature file but coverage file has A, B, C (stale). Coverage file is updated to only have A and B.
  #   5. Developer has coverage file with uncovered 'Scenario X' but deletes the scenario from feature file. Running update-work-unit-status no longer reports 'Scenario X uncovered' error.
  #   6. Developer runs update-scenario to rename 'User logs in' to 'User authenticates'. The coverage file entry is renamed and all test mappings are preserved.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we preserve test/implementation mappings from deleted scenarios in a separate archive file for historical tracking?
  #   A: true
  #
  #   Q: What should happen if a scenario is renamed (appears as both deleted and new)? Should we attempt to preserve mappings based on step similarity?
  #   A: true
  #
  #   Q: Should the fix emit warnings/notifications when stale scenarios are removed from coverage files, or silently clean up?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No, do not preserve mappings from deleted scenarios. Clean removal without archiving.
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to have coverage files automatically sync when scenarios are deleted
    So that I don't get false-positive uncovered scenario errors

  Scenario: Delete scenario removes coverage entry
    Given a feature file test.feature with scenarios A, B, and C
    And a coverage file test.feature.coverage with entries for A, B, and C
    When I run 'fspec delete-scenario test.feature "Scenario B"'
    Then the coverage file should only contain entries for A and C
    And the coverage statistics should show totalScenarios as 2
    And the output should display '✓ Deleted scenario "Scenario B" from test.feature' with coverage update notification


  Scenario: Bulk delete scenarios by tag removes coverage entries
    Given multiple feature files with scenarios tagged @deprecated
    And coverage files exist for all features
    When I run 'fspec delete-scenarios --tag @deprecated'
    Then all @deprecated scenarios should be removed from feature files
    And all corresponding coverage entries should be removed
    And coverage statistics should be recalculated for affected files


  Scenario: Coverage validation detects stale scenarios
    Given a feature file with scenarios A and B
    And a coverage file with entries for A, B, and C (stale)
    When I run 'fspec update-work-unit-status WORK-001 validating'
    Then the command should fail with an error
    And the output should suggest running 'fspec generate-coverage' to sync
    And the output should list the stale scenario names


  Scenario: Generate coverage syncs deleted scenarios
    Given a feature file with scenarios A and B
    And a coverage file with entries for A, B, and C (stale)
    When I run 'fspec generate-coverage'
    Then the coverage file should only contain entries for A and B
    And scenario C should be removed from the coverage file
    And the output should display 'ℹ Removed 1 stale scenario from coverage'


  Scenario: Deleted scenario no longer causes uncovered error
    Given a coverage file with uncovered scenario X
    And scenario X has been deleted from the feature file
    And the coverage file has been synced to remove X
    When I run 'fspec update-work-unit-status WORK-001 validating'
    Then the command should succeed
    And no 'scenario X uncovered' error should be reported


  Scenario: Rename scenario preserves test mappings
    Given a scenario 'User logs in' with test mappings
    When I run 'fspec update-scenario user-login "User logs in" "User authenticates"'
    Then the feature file should show the renamed scenario 'User authenticates'
    And the coverage file should have entry for 'User authenticates'
    And all test mappings should be preserved from the old scenario name

