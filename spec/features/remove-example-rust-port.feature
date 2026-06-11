@done
@RPC-273
Feature: Port remove-example command to Rust

  """
  Core impl file: codelet/fspec-core/src/commands/remove_example.rs — replaces the NotYetPorted stub.
  Public signature `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  Reuses shared infrastructure: io::ensure::ensure_work_units_file, io::locked_file::write_json_atomic, io::time::iso8601_now.
  ExampleItem array lives in WorkUnit.extra["examples"] (since WorkUnit uses #[serde(flatten)]).
  Locate-by-id semantics (NOT by array position). Idempotent re-delete returns success
  WITHOUT disk mutation. Soft-delete sets deleted=true + deletedAt=ISO.
  Two-front-doors: dispatcher and clap CLI both call commands::remove_example::run(args_json, project_root).
  Bridge marshals clap positional args into JSON object {workUnitId, index}. NO domain logic in bridge.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want a Rust port of `remove-example` callable from both dispatchers
    So that the standalone fspec binary can soft-delete Example Mapping examples with parity to TypeScript

  Scenario: Happy-path soft-delete sets the deleted flag and deletedAt
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has two examples id=0 'first' and id=1 'second' both deleted=false
    When I dispatch remove-example with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Removed example: "second"'
    And spec/work-units.json on disk shows AUTH-001.examples[1].deleted=true
    And spec/work-units.json on disk shows AUTH-001.examples[1].deletedAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Idempotent re-delete returns success without writing to disk
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 already deleted=true
    When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Removed example:'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Status guard rejects remove-example when work unit is not in specifying
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one example id=0
    When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Can only remove examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    When I dispatch remove-example with workUnitId='NOPE-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Missing examples array reports 'has no examples'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has no examples field
    When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring 'Work unit AUTH-001 has no examples'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Empty examples array reports 'has no examples'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples=[]
    When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring 'Work unit AUTH-001 has no examples'

  Scenario: Unknown example id reports 'Example with ID <n> not found'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples with id=0 and id=2 (non-contiguous)
    When I dispatch remove-example with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns an error
    And the error message contains the substring 'Example with ID 1 not found'
    And spec/work-units.json on disk is byte-equal to its pre-call contents
