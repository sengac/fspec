@done
@RPC-290
Feature: Port restore-question command to Rust
  """
  Reuses shared infrastructure ensure_work_units_file (load/auto-create), write_json_atomic (atomic write), iso8601_now (timestamp). Questions live in WorkUnit.extra under 'questions' as Value::Array of objects keyed by 'id'.
  Files: replace stub at rust/fspec-core/src/commands/restore_question.rs; NEW rust/fspec-core/src/help/configs/restore_question.rs; NEW rust/fspec/src/restore_question.rs (bridge); NEW rust/fspec-core/tests/restore_question.rs (dispatcher); NEW rust/fspec/tests/cli_restore_question.rs; NEW rust/fspec/tests/fixtures/help/restore-question.txt.
  Two-front-doors: dispatcher AND clap CLI both invoke commands::restore_question::run(args_json, project_root). CLI bridge marshals positional workUnitId + index into JSON {workUnitId, index}.
  Inverse mutation of remove-question (RPC-278): clears deleted flag, REMOVES (not nullifies) the deletedAt key, idempotent when already active. Does NOT update data.meta.lastUpdated (TS parity).
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust implementation of restore-question that matches the TypeScript soft-delete restoration behaviour
    So that the standalone fspec Rust binary can restore soft-deleted Example Mapping questions without depending on Node.js

  Scenario: Dispatcher restores a soft-deleted question by stable ID
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' marked deleted with deletedAt '1999-01-01T00:00:00.000Z'
    When I dispatch restore-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=true
    And the dispatcher output contains restoredQuestion='Q?'
    And the dispatcher output contains activeCount=1
    And spec/work-units.json on disk shows the question with id=0 has deleted=false
    And spec/work-units.json on disk shows the question with id=0 has no deletedAt field

  Scenario: Dispatcher is idempotent when the question is already active
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' deleted=false
    When I capture the exact byte contents of spec/work-units.json
    And I dispatch restore-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=true
    And the dispatcher output contains message='Item ID 0 already active'
    And spec/work-units.json is byte-equal to the previously captured contents

  Scenario: Dispatcher rejects an unknown work unit
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I dispatch restore-question with workUnitId 'MISSING-001' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: Dispatcher rejects restoration when the work unit is not in specifying status
    Given spec/work-units.json contains work unit 'AUTH-001' in 'testing' status with one question id=0 marked deleted
    When I dispatch restore-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only restore questions during discovery/specification phase. AUTH-001 is in 'testing' state."

  Scenario: Dispatcher rejects when the questions array is missing or empty
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no questions array
    When I dispatch restore-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring 'Work unit AUTH-001 has no questions'

  Scenario: Dispatcher rejects when the question ID is not found
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 marked deleted
    When I dispatch restore-question with workUnitId 'AUTH-001' and index 5
    Then the dispatcher returns success=false
    And the error message contains the substring 'Question with ID 5 not found'

  Scenario: Dispatcher computes activeCount as the number of non-deleted questions after restoration
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with three questions ids 0, 1, 2 where ids 0 and 1 are deleted and id 2 is active
    When I dispatch restore-question with workUnitId 'AUTH-001' and index 1
    Then the dispatcher returns success=true
    And the dispatcher output contains activeCount=2
    And spec/work-units.json on disk shows the question with id=1 has deleted=false

  Scenario: Dispatcher fails fast when required args are missing
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch restore-question with no workUnitId field in the args
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid args for fspec command restore-question'
