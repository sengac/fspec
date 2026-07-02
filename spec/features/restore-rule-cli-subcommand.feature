@done
@RPC-291
Feature: fspec restore-rule CLI subcommand
  """
  CLI bridge: codelet/fspec/src/restore_rule.rs — clap variant mirroring TS Commander.js
  at src/commands/restore-rule.ts:122-142. Surface: `fspec restore-rule <workUnitId> <index>`.
  Stdout (success): '✓ Restored rule: "<text>"' (with optional '  Item ID <n> already active'
  second line for idempotent path).
  Stderr (failure): '✗ Failed to restore rule: <message>' with exit code 1.
  TS Commander.js does NOT register `--ids`; the Rust CLI mirrors that exit-1 rejection.
  The bulk-restore code path is dispatcher-only (LLM JSON shape {workUnitId, ids: "..."}).
  Two-front-doors invariant: bridge marshals {workUnitId, index} as JSON and forwards to
  commands::restore_rule::run — NO domain logic.
  Help fixture captured from `node dist/index.js restore-rule --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want `fspec restore-rule` to parse the same positional args as the TypeScript Commander.js registration
    So that TS-CLI-driven scripts continue working after the Rust cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec restore-rule --help`
    Then the exit code is 0
    And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/restore-rule.txt

  Scenario: Happy-path single restore via CLI
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one rule id=0 'r0' deleted=true with a deletedAt timestamp
    When I run `fspec restore-rule AUTH-001 0` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Restored rule: "r0"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[0] has no deletedAt key

  Scenario: Missing work unit exits 1 with canonical stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I run `fspec restore-rule NOPE-001 0` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to restore rule:'
    And stderr contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Wrong status exits 1 with the phase-guard message
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one rule id=0 deleted=true
    When I run `fspec restore-rule AUTH-001 0` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Can only restore rules during discovery/specification phase. AUTH-001 is in 'backlog' state."

  Scenario: Non-numeric index falls through to TS parseInt NaN parity
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one rule id=0 deleted=true
    When I run `fspec restore-rule AUTH-001 abc` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Rule with ID NaN not found'

  Scenario: Unknown --ids flag is rejected by clap with non-zero exit
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 and id=1 both deleted=true
    When I run `fspec restore-rule AUTH-001 0 --ids 1,2` in that tempdir
    Then the exit code is not 0
    And stderr contains the substring 'unknown'

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 and id=1 both deleted=true
    When I dispatch restore-rule via fspec_core::dispatch with workUnitId='AUTH-001' and index=0
    And I run the binary `fspec restore-rule AUTH-001 1` against the same workspace shape
    Then both invocations call commands::restore_rule::run with the same JSON-marshalled args
    And both rules end up deleted=false on disk
