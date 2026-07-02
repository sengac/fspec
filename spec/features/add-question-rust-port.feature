@done
@RPC-188
@rust
@mutation
Feature: Port add-question command to Rust
  """
  Files (replace stub) codelet/fspec-core/src/commands/add_question.rs; NEW codelet/fspec-core/src/help/configs/add_question.rs; NEW codelet/fspec/src/add_question.rs (bridge); NEW codelet/fspec-core/tests/add_question.rs (dispatcher); NEW codelet/fspec/tests/cli_add_question.rs; NEW codelet/fspec/tests/fixtures/help/add-question.txt
  Reuses shared infra: io::ensure::ensure_work_units_file (auto-create), io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now (timestamps). Questions live in WorkUnit.extra under 'questions' / 'nextQuestionId' because the typed WorkUnit struct does not model them yet — round-trips through serde_json::Value preserves all other unknown work-unit fields.
  Mention regex: hand-rolled ASCII scan (no `regex` crate dep). After each @ collect [A-Za-z0-9_]+ characters; mirrors JS /@\w+/g semantics for ASCII inputs.
  Two-front-doors: dispatcher AND clap CLI both call commands::add_question::run(args_json, project_root). CLI bridge marshals positional workUnitId + question into JSON {workUnitId, question}.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-question from the agent loop with byte-for-byte parity to the TypeScript implementation
    So that I can capture Example Mapping questions on work units without depending on Node.js, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Adds a question with @human mention to a specifying work unit
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no questions array
    When I dispatch add-question with workUnitId 'AUTH-001' and question '@human: Support OAuth?'
    Then the dispatcher returns success=true
    And the dispatcher output contains questionCount=1
    And the dispatcher output contains mentionedPeople=['human']
    And spec/work-units.json on disk contains a question with id=0 and text '@human: Support OAuth?'
    And spec/work-units.json on disk contains nextQuestionId=1 on AUTH-001

  Scenario: Rejects add-question when the work unit does not exist
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I dispatch add-question with workUnitId 'AUTH-999' and question 'Q?'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-999' does not exist"

  Scenario: Rejects add-question when the work unit is not in specifying status
    Given spec/work-units.json contains work unit 'AUTH-001' in 'backlog' status
    When I dispatch add-question with workUnitId 'AUTH-001' and question 'Q?'
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only add questions during discovery/specification phase. AUTH-001 is in 'backlog' state."

  Scenario: Honors existing nextQuestionId by reusing it and bumping by one
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with nextQuestionId=3
    When I dispatch add-question with workUnitId 'AUTH-001' and question 'New question'
    Then the dispatcher returns success=true
    And the dispatcher output contains questionCount=1
    And spec/work-units.json on disk contains a question with id=3
    And spec/work-units.json on disk contains nextQuestionId=4 on AUTH-001

  Scenario: Omits mentionedPeople when no @mention is present in the question
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    When I dispatch add-question with workUnitId 'AUTH-001' and question 'Should we add caching?'
    Then the dispatcher returns success=true
    And the dispatcher output does NOT contain the field 'mentionedPeople'

  Scenario: Preserves auxiliary work unit fields on round-trip write
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with auxiliary rules, examples, and virtualHooks arrays
    When I dispatch add-question with workUnitId 'AUTH-001' and question 'Q?'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk still contains the original rules array
    And spec/work-units.json on disk still contains the original examples array
    And spec/work-units.json on disk still contains the original virtualHooks array

  Scenario: Initializes missing nextQuestionId to 0 for backward compatibility
    Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no nextQuestionId field
    When I dispatch add-question with workUnitId 'AUTH-001' and question 'First question'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk contains a question with id=0
    And spec/work-units.json on disk contains nextQuestionId=1 on AUTH-001
