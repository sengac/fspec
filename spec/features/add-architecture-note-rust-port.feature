@done
@RPC-168
@rust
@cli
@mutation
Feature: Port add-architecture-note command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_architecture_note.rs uses ensure_work_units_file to load (or auto-create) spec/work-units.json, validates that the requested work unit exists, appends a new ArchitectureNoteItem object to workUnit.architectureNotes with the literal field order `id, text, deleted, createdAt`, increments workUnit.nextNoteId, bumps workUnit.updatedAt and data.meta.lastUpdated, and persists via io::locked_file::write_json_atomic so that prefixCounters, migrationHistory and other unknown top-level fields round-trip losslessly.
  Help config at codelet/fspec-core/src/help/configs/add_architecture_note.rs mirrors src/commands/add-architecture-note-help.ts byte-for-byte.
  CLI bridge at codelet/fspec/src/add_architecture_note.rs marshals the two positional args (workUnitId, note) into JSON and delegates to commands::add_architecture_note::run. No domain logic is duplicated.
  Two-front-doors: clap CLI and LLM dispatcher both call commands::add_architecture_note::run(args_json, project_root).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust implementation of the add-architecture-note command that matches the TypeScript behaviour exactly
    So that the standalone fspec Rust binary can append architecture notes without depending on Node.js

  Scenario: Dispatcher appends a new architecture note to an existing work unit
    Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Uses bcrypt for password hashing'
    Then the dispatcher returns success=true
    And spec/work-units.json work unit 'AUTH-001' has exactly one architectureNote
    And that note has id=0, text='Uses bcrypt for password hashing', and deleted=false
    And work unit 'AUTH-001' has nextNoteId=1

  Scenario: Dispatcher increments nextNoteId on each invocation
    Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Note A'
    And I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Note B'
    Then spec/work-units.json work unit 'AUTH-001' has two architectureNotes
    And the second architecture note has id=1
    And work unit 'AUTH-001' has nextNoteId=2

  Scenario: Dispatcher rejects unknown work unit IDs
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I dispatch add-architecture-note with workUnitId='MISSING-001' and note='anything'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-architecture-note with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command add-architecture-note'

  Scenario: Dispatcher response data contains the canonical success line and system reminder
    Given spec/work-units.json contains work unit 'AUTH-001'
    When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Uses bcrypt'
    Then the DispatchResult.data contains the line '✓ Architecture note added successfully'
    And the DispatchResult.data contains the substring '<system-reminder>'
    And the DispatchResult.data contains the substring 'ARCHITECTURE NOTE ADDED'
    And the DispatchResult.data contains the substring '"Uses bcrypt"'

  Scenario: Dispatcher preserves unknown top-level fields on write
    Given spec/work-units.json contains work unit 'AUTH-001' and a top-level 'prefixCounters' object
    When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='note'
    Then the dispatcher returns success=true
    And spec/work-units.json still contains the top-level 'prefixCounters' object

  Scenario: Dispatcher auto-creates spec/work-units.json when missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='note'
    Then the file spec/work-units.json exists
    And the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
