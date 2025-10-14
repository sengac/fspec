@COV-007
@done
@file-ops
@coverage-tracking
@cli
@phase2
Feature: Generate Coverage Files for Existing Features
  """
  Architecture notes:
  - New command 'fspec generate-coverage' to scan spec/features/ for .feature files without .coverage files
  - Reuse createCoverageFile() utility from src/utils/coverage-file.ts
  - Parse each .feature file with @cucumber/gherkin to extract scenario names
  - Generate .coverage files with same JSON schema as auto-created coverage files
  - Support --dry-run flag for preview without file creation
  - Display summary: X created, Y skipped, Z recreated
  - Exit code 0 on success, 1 on errors
  """

  Background: User Story
    As a developer using fspec for coverage tracking
    I want to generate coverage files for existing feature files that lack them
    So that I can start tracking test coverage for features created before coverage tracking was implemented

  Scenario: Generate coverage files for all features without coverage
    Given I have a project with spec/features/ directory
    And there are 3 .feature files with no .coverage files
    When I run 'fspec generate-coverage'
    Then 3 .feature.coverage files should be created
    And the output should display 'Created 3 coverage files'

  Scenario: Skip existing valid coverage files
    Given I have 5 .feature files
    And 3 of them already have valid .feature.coverage files
    When I run 'fspec generate-coverage'
    Then 2 new .feature.coverage files should be created
    And the 3 existing files should remain unchanged
    And the output should display 'Created 2, Skipped 3'

  Scenario: Recreate corrupted coverage files with invalid JSON
    Given I have a .feature file user-login.feature
    And a corrupted user-login.feature.coverage file with invalid JSON
    When I run 'fspec generate-coverage'
    Then the corrupted file should be overwritten with valid JSON
    And the output should display 'Recreated 1 (invalid JSON)'

  Scenario: Dry-run mode previews without creating files
    Given I have 3 .feature files with no .coverage files
    When I run 'fspec generate-coverage --dry-run'
    Then no .coverage files should be created
    And the output should display 'Would create 3 coverage files (DRY RUN)'
    And the output should list the filenames that would be created
