@INIT-002
@phase1
@critical
@cli
@project-management
@initialization
@file-ops
Feature: Automatic JSON File Initialization
  """
  Architecture notes:
  - ALL commands that read JSON files MUST use ensure utilities
  - ensure utilities: ensureWorkUnitsFile, ensurePrefixesFile, ensureEpicsFile
  - Located in src/utils/ensure-files.ts
  - Auto-creates missing files with proper initial structure
  - Ensures spec/ directory exists before creating files

  Critical implementation requirements:
  - MUST NOT fail with ENOENT errors when files missing
  - MUST create files with proper JSON schema structure
  - MUST be idempotent (safe to call multiple times)
  - MUST maintain backward compatibility with existing files
  - MUST NOT overwrite existing data

  Files that need automatic initialization:
  - spec/work-units.json (work units and Kanban states)
  - spec/prefixes.json (work unit prefixes)
  - spec/epics.json (epic definitions)
  - spec/tags.json (already handled by register-tag)
  - spec/foundation.json (already handled by add-diagram)

  Problem being solved:
  - Currently 48+ commands directly call readFile() without initialization
  - Commands fail with "ENOENT: no such file or directory" errors
  - Users forced to manually create JSON files or run specific init commands
  - Example mapping commands fail if work-units.json doesn't exist
  - Poor user experience for first-time users

  References:
  - Utility location: src/utils/ensure-files.ts
  - Example usage: src/commands/list-work-units.ts (line 29)
  - Example usage: src/commands/create-work-unit.ts
  """

  Background: User Story
    As an AI agent using fspec for project management
    I want all commands to automatically initialize required JSON files
    So that I never encounter file-not-found errors when using fspec commands

  @critical
  @happy-path
  @COV-010
  Scenario: Example mapping commands auto-create work-units.json
    Given I have a fresh project with only spec/features/ directory
    And spec/work-units.json does not exist
    When I run "fspec add-example work-unit-query 'Query by status' 'Example data'"
    Then the command should succeed
    And spec/work-units.json should be created with proper structure
    And the file should contain empty workUnits object
    And the file should contain all Kanban states

  @critical
  @happy-path
  @COV-011
  Scenario: Dependency commands auto-create work-units.json
    Given I have a fresh project with only spec/features/ directory
    And spec/work-units.json does not exist
    When I run any dependency management command
    Then the command should auto-create spec/work-units.json
    And should not fail with ENOENT error

  @critical
  @happy-path
  Scenario: Epic commands auto-create epics.json
    Given I have a fresh project
    And spec/epics.json does not exist
    When I run "fspec create-epic user-management 'User Management'"
    Then the command should succeed
    And spec/epics.json should be created with empty epics object

  @critical
  @happy-path
  Scenario: Prefix commands auto-create prefixes.json
    Given I have a fresh project
    And spec/prefixes.json does not exist
    When I run "fspec create-prefix AUTH 'Authentication features'"
    Then the command should succeed
    And spec/prefixes.json should be created with empty prefixes object

  @idempotent
  Scenario: Calling ensure utilities multiple times is safe
    Given I have spec/work-units.json with existing work units
    When ensureWorkUnitsFile is called
    Then it should return the existing data
    And should not modify the file
    And should not lose any data

  @validation
  @COV-012
  Scenario: Ensure utilities validate JSON structure
    Given I have corrupted spec/work-units.json
    When ensureWorkUnitsFile is called
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

  @refactoring
  @technical-debt
  @COV-013
  Scenario: All 48+ commands use ensure utilities
    Given I have analyzed all commands in src/commands/
    When I check which commands read JSON files
    Then ALL commands should import from ensure-files
    And NO commands should directly readFile work-units.json without ensure
    And NO commands should directly readFile epics.json without ensure
    And NO commands should directly readFile prefixes.json without ensure

  @init-001
  Scenario: Create epic command auto-creates spec/epics.json when missing
    Given I have a fresh project with spec/ directory
    And spec/epics.json does not exist
    When I run "fspec create-epic user-auth 'User Authentication'"
    Then the command should succeed
    And spec/epics.json should be created
    And the file should contain an empty epics object
    And the epic "user-auth" should be added to the file

  @init-001
  Scenario: Create prefix command auto-creates spec/prefixes.json when missing
    Given I have a fresh project with spec/ directory
    And spec/prefixes.json does not exist
    When I run "fspec create-prefix AUTH 'Authentication features'"
    Then the command should succeed
    And spec/prefixes.json should be created
    And the file should contain an empty prefixes object
    And the prefix "AUTH" should be added to the file

  @init-001
  @COV-014
  Scenario: List work units command auto-creates spec/work-units.json when missing
    Given I have a fresh project with spec/ directory
    And spec/work-units.json does not exist
    When I run "fspec list-work-units"
    Then the command should succeed
    And spec/work-units.json should be created with proper structure
    And the file should contain empty workUnits object
    And the file should contain all 7 Kanban states

  @init-001
  @COV-015
  Scenario: Update work unit command uses ensureWorkUnitsFile instead of direct readFile
    Given I have a fresh project with spec/ directory
    And spec/work-units.json does not exist
    And I have created a work unit "AUTH-001"
    When I run "fspec update-work-unit AUTH-001 --title='New Title'"
    Then the command should succeed
    And spec/work-units.json should exist
    And the work unit "AUTH-001" title should be "New Title"
    And no ENOENT error should occur

  @init-001
  Scenario: Calling ensureWorkUnitsFile multiple times returns same data without overwriting
    Given I have spec/work-units.json with existing work unit "AUTH-001"
    And the work unit has title "Original Title"
    When ensureWorkUnitsFile is called multiple times
    Then it should return the existing data each time
    And the work unit title should still be "Original Title"
    And the file should not be modified or overwritten

  @COV-016
  Scenario: Register tag command auto-creates spec/tags.json when missing
    Given I have a fresh project with spec/ directory
    Given spec/tags.json does not exist
    When I run "fspec register-tag @my-tag 'Phase Tags' 'My custom tag'"
    Then the command should succeed
    And spec/tags.json should be created
    And the file should contain valid Tags JSON structure with default categories
    And the tag @my-tag should be added to the Phase Tags category

  @COV-017
  Scenario: Update foundation command auto-creates spec/foundation.json when missing
    Given I have a fresh project with spec/ directory
    Given spec/foundation.json does not exist
    When I run "fspec update-foundation projectOverview 'My project overview'"
    Then the command should succeed
    And spec/foundation.json should be created
    And the file should contain valid Foundation JSON structure

  @COV-018
  Scenario: List tags command auto-creates spec/tags.json instead of throwing error
    Given I have a fresh project with spec/ directory
    Given spec/tags.json does not exist
    When I run "fspec list-tags"
    Then the command should not throw "tags.json not found" error
    And the command should succeed
    And spec/tags.json should be auto-created with default structure

  @COV-019
  Scenario: Show foundation command auto-creates spec/foundation.json instead of returning error
    Given I have a fresh project with spec/ directory
    Given spec/foundation.json does not exist
    When I run "fspec show-foundation"
    Then the command should not return "foundation.json not found" error
    And the command should succeed
    And spec/foundation.json should be auto-created with default structure
