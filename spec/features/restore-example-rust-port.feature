@done
@RPC-289
Feature: Port restore-example command to Rust
  """
  Core impl: codelet/fspec-core/src/commands/restore_example.rs — replaces the NotYetPorted stub.
  Single source of truth `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  Reuses io::ensure::ensure_work_units_file, io::locked_file::write_json_atomic, io::time::iso8601_now.
  ExampleItem array lives in WorkUnit.extra["examples"] (since WorkUnit uses #[serde(flatten)]).
  Locate-by-stable-id semantics. Idempotent re-restore returns success WITHOUT disk mutation.
  Happy-path restore: deleted=false, removes the deletedAt key (parity with TS `delete example.deletedAt`),
  bumps workUnit.updatedAt. Two-front-doors: dispatcher and clap CLI both call
  commands::restore_example::run(args_json, project_root). Bridge marshals positional args
  into JSON {workUnitId, index}. NO domain logic in bridge.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want a Rust port of `restore-example` callable from both dispatchers
    So that the standalone fspec binary can un-soft-delete Example Mapping examples with byte parity to the TypeScript shell

  Scenario: Happy-path restore clears the deleted flag and removes deletedAt
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has two examples id=0 'first' deleted=true with a deletedAt timestamp and id=1 'second' deleted=false
    When I dispatch restore-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored example: "first"'
    And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.examples[0] has no deletedAt key
    And spec/work-units.json on disk shows AUTH-001.examples[1].deleted=false
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Idempotent re-restore returns success without writing to disk
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 'text' already deleted=false
    When I dispatch restore-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored example: "text"'
    And the rendered output contains the substring 'Item ID 0 already active'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    When I dispatch restore-example with workUnitId='NOPE-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Status guard rejects restore-example when work unit is not in specifying
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one example id=0 deleted=true
    When I dispatch restore-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Can only restore examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing examples array reports 'has no examples'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has no examples field
    When I dispatch restore-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring 'Work unit AUTH-001 has no examples'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Unknown example id reports 'Example with ID <n> not found'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples with id=0 and id=2 (non-contiguous, no id=1)
    When I dispatch restore-example with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns an error
    And the error message contains the substring 'Example with ID 1 not found'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Auto-creates spec/work-units.json when missing then reports the canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch restore-example with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure
