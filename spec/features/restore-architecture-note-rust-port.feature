@done
@RPC-287
Feature: Port restore-architecture-note command to Rust

  """
  Reuses shared infrastructure ensure_work_units_file (load/auto-create), write_json_atomic (atomic write), iso8601_now (timestamp). Architecture notes live in WorkUnit.extra under 'architectureNotes' as Value::Array of objects keyed by 'id'.
  Files: replace stub at codelet/fspec-core/src/commands/restore_architecture_note.rs; NEW codelet/fspec-core/src/help/configs/restore_architecture_note.rs; NEW codelet/fspec/src/restore_architecture_note.rs (bridge); NEW codelet/fspec-core/tests/restore_architecture_note.rs (dispatcher); NEW codelet/fspec/tests/cli_restore_architecture_note.rs; NEW codelet/fspec/tests/fixtures/help/restore-architecture-note.txt.
  Two-front-doors: dispatcher AND clap CLI both invoke commands::restore_architecture_note::run(args_json, project_root). CLI bridge marshals positional workUnitId + index into JSON {workUnitId, index}.
  Inverse mutation of remove-architecture-note (RPC-267): clears deleted flag, REMOVES (not nullifies) the deletedAt key, idempotent when already active. NO status gate. Updates BOTH workUnit.updatedAt AND data.meta.lastUpdated (TS parity).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust implementation of restore-architecture-note that matches the TypeScript soft-delete restoration behaviour
    So that the standalone fspec Rust binary can restore soft-deleted architecture notes without depending on Node.js


  Scenario: Dispatcher restores a soft-deleted architecture note by stable ID
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 text 'Note A' marked deleted with deletedAt '1999-01-01T00:00:00.000Z'
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And the dispatcher output contains restoredNote='Note A'
    And the dispatcher output contains activeCount=1
    And spec/work-units.json on disk shows the architectureNote with id=0 has deleted=false
    And spec/work-units.json on disk shows the architectureNote with id=0 has no deletedAt field


  Scenario: Dispatcher is idempotent when the note is already active
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 text 'Note A' deleted=false
    When I capture the exact byte contents of spec/work-units.json
    And I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And the dispatcher output contains message='Item ID 0 already active'
    And spec/work-units.json is byte-equal to the previously captured contents


  Scenario: Dispatcher rejects an unknown work unit
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I dispatch restore-architecture-note with workUnitId='MISSING-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-001' does not exist"


  Scenario: Dispatcher rejects when architectureNotes is missing or empty
    Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes field
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' has no architecture notes"


  Scenario: Dispatcher rejects when the architecture note ID is not found
    Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=5
    Then the dispatcher returns success=false
    And the error message contains the substring 'Architecture note with ID 5 not found'


  Scenario: Dispatcher computes activeCount as the number of non-deleted notes after restoration
    Given spec/work-units.json contains work unit 'AUTH-001' with three architectureNotes ids 0, 1, 2 where ids 0 and 1 are deleted and id 2 is active
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns success=true
    And the dispatcher output contains activeCount=2
    And spec/work-units.json on disk shows the architectureNote with id=1 has deleted=false


  Scenario: Dispatcher allows restoration regardless of work unit status
    Given spec/work-units.json contains work unit 'AUTH-001' in 'done' status with one architectureNote id=0 marked deleted
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows the architectureNote with id=0 has deleted=false


  Scenario: Dispatcher updates meta.lastUpdated as well as workUnit.updatedAt
    Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted and meta.lastUpdated '1999-01-01T00:00:00.000Z'
    When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success=true
    And spec/work-units.json on disk has meta.lastUpdated NOT equal to '1999-01-01T00:00:00.000Z'
    And spec/work-units.json on disk has workUnits.'AUTH-001'.updatedAt NOT equal to '1999-01-01T00:00:00.000Z'


  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch restore-architecture-note with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command restore-architecture-note'
