@version-management
@critical
@file-ops
@migration
@versioning
@BUG-070
Feature: work-units.json not stamped with current version on initial creation
  """
  Current implementation has CURRENT_VERSION defined separately in src/utils/ensure-files.ts and DEFAULT_VERSION in src/migrations/registry.ts - this violates DRY
  Solution: Create src/utils/version.ts with single export const CURRENT_VERSION = '0.7.0', import from both files
  Alternatively: Export CURRENT_VERSION from migrations/registry.ts and import in ensure-files.ts (reuse existing constant)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New work-units.json files MUST be stamped with current version immediately on creation
  #   2. Version string MUST be managed using DRY principles - NO version string repetition across files
  #   3. MUST create/use a shared version constant that both ensure-files.ts and migrations use
  #   4. The single source of truth for current version MUST be accessible to all version-dependent code
  #   5. Hardcoded version strings like '0.7.0' MUST NOT appear in multiple files
  #
  # EXAMPLES:
  #   1. User creates new project, runs first fspec command, work-units.json created with version: '0.7.0', no migration runs
  #   2. User deletes spec/work-units.json, runs fspec command, file recreated with current version, no backup created
  #   3. Version constant updated to 0.8.0 in shared file, both ensure-files and migrations automatically use new version
  #
  # ========================================
  Background: User Story
    As a developer using fspec for the first time
    I want to initialize a new project with work-units.json
    So that I don't see unnecessary migration warnings or backup files

  Scenario: Create work-units.json with current version on first run
    Given I am in a new project without spec/work-units.json
    When I run the first fspec command
    Then spec/work-units.json should be created
    And the file should have version field set to '0.7.1'
    And no migration should run
    And no backup files should be created

  Scenario: Recreate work-units.json with current version after deletion
    Given I have an existing fspec project
    And spec/work-units.json has been deleted
    When I run any fspec command
    Then spec/work-units.json should be recreated
    And the file should have version field set to '0.7.1'
    And no migration should run
    And no backup files should be created

  Scenario: Version constant shared across ensure-files and migrations
    Given the version constant is defined in a shared location
    When the version is updated to '0.8.0' in the shared constant
    Then ensure-files.ts should use version '0.8.0' when creating work-units.json
    And migrations should recognize '0.8.0' as the current version
    And no version string should be hardcoded in multiple files
