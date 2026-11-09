@COV-019
@COV-018
@COV-017
@COV-016
@COV-015
@COV-014
@COV-013
@COV-012
@COV-011
@COV-010
@INIT-002
@critical
@cli
@project-management
@initialization
@file-ops
Feature: Automatic JSON File Initialization
  """
  Architecture notes:
  - Commands load spec state files through the ensure helpers in rust/fspec-core/src/io/ensure.rs: ensure_work_units_file, ensure_prefixes_file, ensure_tags_file, ensure_foundation_file, ensure_epics_file
  - A missing file is created (with the spec/ directory) holding its canonical initial structure; an existing file is read and returned unchanged
  - A malformed file is a parse error naming the file, never silently overwritten
  - work-units.json starts with version, meta.version, meta.lastUpdated, an empty workUnits object and the 7 Kanban state arrays, all empty

  Per-command auto-creation is proved in each command's own feature (add-example, add-dependency, create-epic, create-prefix, list-work-units, register-tag, list-tags, show-foundation, update-foundation and update-work-unit rust ports); this feature covers the work-units helper itself (tests: rust/fspec-core/src/io/ensure/init_tests.rs).
  """

  Background: User Story
    As an AI agent using fspec for project management
    I want all commands to automatically initialize required JSON files
    So that I never encounter file-not-found errors when using fspec commands

  @idempotent
  Scenario: Calling ensure utilities multiple times is safe
    Given I have spec/work-units.json with existing work units
    When ensure_work_units_file is called
    Then it should return the existing data
    And should not modify the file
    And should not lose any data

  @validation
  Scenario: Ensure utilities validate JSON structure
    Given I have corrupted spec/work-units.json
    When ensure_work_units_file is called
    Then it should throw a helpful error
    And should indicate the file is invalid JSON

  @file-structure
  Scenario: Auto-created files have proper structure
    Given I have a fresh project
    When work-units.json is auto-created
    Then it should have meta.version field
    Then it should have meta.lastUpdated timestamp
    And it should have workUnits empty object
    And it should have states with all 7 Kanban states
    And states should each be empty arrays

  @init-001
  Scenario: Calling ensure_work_units_file multiple times returns same data without overwriting
    Given I have spec/work-units.json with existing work unit "AUTH-001"
    And the work unit has title "Original Title"
    When ensure_work_units_file is called multiple times
    Then it should return the existing data each time
    And the work unit title should still be "Original Title"
    And the file should not be modified or overwritten
