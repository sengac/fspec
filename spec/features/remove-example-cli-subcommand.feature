@done
@RPC-273
Feature: fspec remove-example CLI subcommand

  """
  CLI bridge: codelet/fspec/src/remove_example.rs — clap variant mirroring TS Commander.js
  at src/commands/remove-example.ts:88-107. Surface: `fspec remove-example <workUnitId> <index>`.
  Stdout (success): '✓ Removed example: "<text>"' OR 'Item ID <n> already deleted'.
  Stderr (failure): '✗ Failed to remove example: <message>' with exit code 1.
  Two-front-doors invariant: bridge marshals {workUnitId, index} as JSON and forwards to
  commands::remove_example::run — NO domain logic.
  Help fixture captured from `node dist/index.js remove-example --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want `fspec remove-example` to parse the same positional args as the TypeScript Commander.js registration
    So that TS-CLI-driven scripts continue working after the Rust cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-example --help`
    Then the exit code is 0
    And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-example.txt

  Scenario: Happy-path soft-delete via CLI
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 'hello'
    When I run `fspec remove-example AUTH-001 0` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed example: "hello"'
    And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=true

  Scenario: Missing work unit exits 1 with canonical stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I run `fspec remove-example NOPE-001 0` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to remove example:'
    And stderr contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Wrong status exits 1 with the phase-guard message
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one example id=0
    When I run `fspec remove-example AUTH-001 0` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Can only remove examples during discovery/specification phase. AUTH-001 is in 'backlog' state."

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples id=0 and id=1
    When I dispatch remove-example via fspec_core::dispatch with workUnitId='AUTH-001' and index=0
    And I run the binary `fspec remove-example AUTH-001 1` against the same workspace shape
    Then both invocations call commands::remove_example::run with the same JSON-marshalled args
    And both examples end up deleted=true on disk
