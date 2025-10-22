@done
@COV-008
@modification
@coverage-tracking
@cli
Feature: Unlink Coverage Mappings
  """
  Architecture notes:
  - Command: fspec unlink-coverage <feature-name> with options
  - Loads .feature.coverage file, finds scenario
  - Supports three modes: --all (remove everything), --test-file (remove test + impl), --test-file + --impl-file (remove only impl)
  - Recalculates stats after removal
  - Writes updated coverage file
  - Exit code 0 on success, 1 on errors
  """

  Background: User Story
    As a developer managing coverage tracking
    I want to remove incorrect or outdated test and implementation mappings from scenarios
    So that I can correct mistakes and update coverage as code evolves without manual JSON editing

  Scenario: Remove all mappings from scenario with --all flag
    Given I have a scenario with test and implementation mappings
    When I run 'fspec unlink-coverage user-login --scenario "Login" --all'
    Then all mappings should be removed
    And the scenario should have empty testMappings array
    And stats should show coveragePercent decreased

  Scenario: Remove test mapping removes implementation mappings too
    Given I have a test mapping with implementation mappings
    When I run 'fspec unlink-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts'
    Then the test mapping and all its impl mappings should be removed

  Scenario: Remove only implementation mapping keeps test mapping
    Given I have a test mapping with an implementation mapping
    When I run 'fspec unlink-coverage user-login --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/old.ts'
    Then only the implementation mapping should be removed
    And the test mapping should still exist
