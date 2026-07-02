@done
@RPC-267
@rust
@cli
@mutation
Feature: Port remove-architecture-note command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/remove_architecture_note.rs uses ensure_work_units_file to load (or auto-create) spec/work-units.json, validates that the requested work unit exists and has architecture notes, looks up the note by its STABLE id (not array position), soft-deletes the matched note by setting deleted=true plus deletedAt=iso8601_now(), bumps workUnit.updatedAt and data.meta.lastUpdated, and persists via io::locked_file::write_json_atomic. When the matched note is already deleted, the function returns an idempotent success WITHOUT mutating disk and surfaces the canonical "Item ID <id> already deleted" message.
  Help config at codelet/fspec-core/src/help/configs/remove_architecture_note.rs mirrors src/commands/remove-architecture-note-help.ts byte-for-byte.
  CLI bridge at codelet/fspec/src/remove_architecture_note.rs marshals two positional args (workUnitId, index:u32) into JSON and delegates to commands::remove_architecture_note::run. No domain logic is duplicated.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::remove_architecture_note::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust implementation of the remove-architecture-note command that matches the TypeScript soft-delete and idempotent behaviour
    So that the standalone fspec Rust binary can soft-delete architecture notes without depending on Node.js

  Scenario: Dispatcher soft-deletes the matching architecture note by ID
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNotes ids 0 and 1
    When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-001' architectureNotes[0] has deleted=true
    And spec/work-units.json work unit 'AUTH-001' architectureNotes[0] has a non-empty deletedAt
    And spec/work-units.json work unit 'AUTH-001' architectureNotes[1] still has deleted=false

  Scenario: Dispatcher is idempotent on already-deleted notes
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 already deleted
    When I capture the exact byte contents of spec/work-units.json
    And I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And the DispatchResult.data contains the substring 'Item ID 0 already deleted'
    And spec/work-units.json is byte-equal to the previously captured contents

  Scenario: Dispatcher rejects missing work unit IDs
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I dispatch remove-architecture-note with workUnitId='MISSING-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: Dispatcher rejects when architectureNotes is missing or empty
    Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes field
    When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' has no architecture notes"

  Scenario: Dispatcher rejects an unknown architecture note ID
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNotes ids 0 and 2
    When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns success=false
    And the error message contains the substring 'Architecture note with ID 1 not found'

  Scenario: Dispatcher response data contains the canonical success line
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0
    When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    Then the DispatchResult.data contains the line '✓ Architecture note removed successfully'

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch remove-architecture-note with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command remove-architecture-note'
