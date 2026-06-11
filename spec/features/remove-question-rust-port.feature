@done
@RPC-278
@rust
@mutation
Feature: Port remove-question command to Rust

  """
  Files (replace stub) codelet/fspec-core/src/commands/remove_question.rs; NEW codelet/fspec-core/src/help/configs/remove_question.rs; NEW codelet/fspec/src/remove_question.rs (bridge); NEW codelet/fspec-core/tests/remove_question.rs (dispatcher); NEW codelet/fspec/tests/cli_remove_question.rs; NEW codelet/fspec/tests/fixtures/help/remove-question.txt
  Reuses shared infra: io::ensure::ensure_work_units_file (auto-create), io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now (timestamps). Questions live in WorkUnit.extra under 'questions' as Value::Array; lookup is by id field, NOT by positional offset.
  Two-front-doors: dispatcher AND clap CLI both call commands::remove_question::run(args_json, project_root). CLI bridge marshals positional workUnitId + index into JSON {workUnitId, index}.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch remove-question from the agent loop with byte-for-byte parity to the TypeScript implementation
    So that I can soft-delete Example Mapping questions without depending on Node.js, sharing one source of truth between the LLM dispatcher and the CLI


  Scenario: Soft-deletes a question by stable ID
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' not deleted
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=true
    And the dispatcher output contains removedQuestion='Q?'
    And the dispatcher output contains remainingCount=0
    And spec/work-units.json on disk shows the question with id=0 has deleted=true
    And spec/work-units.json on disk shows the question with id=0 has a deletedAt timestamp


  Scenario: Rejects remove-question when the work unit does not exist
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I dispatch remove-question with workUnitId 'AUTH-999' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-999' does not exist"


  Scenario: Rejects remove-question when the work unit is not in specifying status
    Given spec/work-units.json contains work unit 'AUTH-001' in 'testing' status with one question id=0
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only remove questions during discovery/specification phase. AUTH-001 is in 'testing' state."


  Scenario: Rejects remove-question when the work unit has no questions
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no questions array
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=false
    And the error message contains the substring 'Work unit AUTH-001 has no questions'


  Scenario: Rejects remove-question when the question ID is not found
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 5
    Then the dispatcher returns success=false
    And the error message contains the substring 'Question with ID 5 not found'


  Scenario: Returns idempotent success when the question is already deleted
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 already soft-deleted with deletedAt '1999-01-01T00:00:00.000Z'
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    Then the dispatcher returns success=true
    And the dispatcher output contains message='Item ID 0 already deleted'
    And spec/work-units.json on disk shows the question with id=0 still has deletedAt='1999-01-01T00:00:00.000Z'


  Scenario: Counts only non-deleted questions in remainingCount
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with three questions ids 0, 1, 2 (none deleted)
    When I dispatch remove-question with workUnitId 'AUTH-001' and index 1
    Then the dispatcher returns success=true
    And the dispatcher output contains remainingCount=2
    And spec/work-units.json on disk contains 3 question records
    And spec/work-units.json on disk shows the question with id=1 has deleted=true
