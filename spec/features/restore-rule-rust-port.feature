@done
@RPC-291
Feature: Port restore-rule command to Rust

  """
  Core impl: codelet/fspec-core/src/commands/restore_rule.rs — replaces the NotYetPorted stub.
  Single source of truth `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  Reuses io::ensure::ensure_work_units_file, io::locked_file::write_json_atomic, io::time::iso8601_now.
  RuleItem array lives in WorkUnit.extra["rules"] (since WorkUnit uses #[serde(flatten)]).
  Two-front-doors: dispatcher and clap CLI both call commands::restore_rule::run.
  Dispatcher front-door supports BOTH single ({workUnitId, index}) and bulk ({workUnitId, ids: "0,1,2"}) shapes;
  when ids is present it takes the bulk branch (TS line 53 short-circuit).
  CLI bridge supports ONLY single positional `<workUnitId> <index>`; the TS Commander.js does not
  register --ids and neither does the Rust clap derive. The bulk surface is dispatcher-only.
  Bulk path: silently skip already-active items, throw 'Rule with ID <n> not found' on unknown id
  BEFORE the single atomic write (no partial restore lands on disk). All-already-active still
  writes updatedAt. Idempotent single re-restore returns success WITHOUT disk mutation.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want a Rust port of `restore-rule` callable from both dispatchers
    So that the standalone fspec binary (single positional restore) and the LLM dispatcher (which also supports bulk `ids` restore) reach parity with the TypeScript implementation

  Scenario: Single-restore happy path clears the deleted flag and removes deletedAt
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has two rules id=0 'r0' deleted=true with a deletedAt timestamp and id=1 'r1' deleted=false
    When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: "r0"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[0] has no deletedAt key
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: Idempotent single re-restore returns success without writing to disk
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one rule id=0 'already active' deleted=false
    When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: "already active"'
    And the rendered output contains the substring 'Item ID 0 already active'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing work unit surfaces the canonical error
    Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    When I dispatch restore-rule with workUnitId='NOPE-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Status guard rejects restore-rule when work unit is not in specifying
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one rule id=0 deleted=true
    When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring "Can only restore rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Missing rules array reports 'has no rules'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has no rules field
    When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    Then the dispatcher returns an error
    And the error message contains the substring 'Work unit AUTH-001 has no rules'

  Scenario: Unknown single rule id reports 'Rule with ID <n> not found'
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules with id=0 and id=2 only
    When I dispatch restore-rule with workUnitId='AUTH-001' and index=1
    Then the dispatcher returns an error
    And the error message contains the substring 'Rule with ID 1 not found'

  Scenario: Bulk-restore happy path restores all listed deleted rules in one atomic write
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0', id=1 'r1' and id=2 'r2' all deleted=true
    When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1,2'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: "r0, r1, r2"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[2].deleted=false

  Scenario: Bulk-restore silently skips already-active items
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true, id=1 'r1' deleted=false and id=2 'r2' deleted=true
    When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1,2'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: "r0, r2"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[2].deleted=false

  Scenario: Bulk-restore fails atomically on unknown id without writing
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true and id=1 'r1' deleted=true
    When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,99,1'
    Then the dispatcher returns an error
    And the error message contains the substring 'Rule with ID 99 not found'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Bulk-restore with all-already-active still bumps updatedAt
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=false and id=1 'r1' deleted=false
    When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: ""'
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: ids takes precedence over index when both are provided
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true and id=1 'r1' deleted=true
    When I dispatch restore-rule with workUnitId='AUTH-001' index=0 and ids='1'
    Then the dispatcher returns success
    And the rendered output contains the substring '✓ Restored rule: "r1"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true
    And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
