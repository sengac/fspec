@done
@RPC-181
Feature: fspec add-example CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_example.rs — clap variant mirroring TS Commander.js registration
  at src/commands/add-example.ts:98-115. Surface: `fspec add-example <workUnitId> <example>`.
  Stdout (success): "✓ Example added successfully\n\n<system-reminder>...</system-reminder>".
  Stderr (failure): "✗ Failed to add example: <message>" line; exit code 1.
  Two-front-doors invariant: bridge marshals clap positional args to JSON {workUnitId, example}
  and forwards to commands::add_example::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-example --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want `fspec add-example` to parse the same positional args as the TypeScript Commander.js registration
    So that any TS-CLI-driven script keeps working after the Rust cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-example --help`
    Then the exit code is 0
    And stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-example.txt

  Scenario: Happy-path invocation marshals positional args and writes the example
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-example AUTH-001 "Valid login"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring "✓ Example added successfully"
    And stdout contains the substring "<system-reminder>"
    And spec/work-units.json on disk shows AUTH-001.examples has length 1

  Scenario: Missing work unit exits 1 with the canonical error on stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I run `fspec add-example NOPE-001 "x"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "✗ Failed to add example:"
    And stderr contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Status guard exits 1 with the phase-guard message on stderr
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec add-example AUTH-001 "x"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-example via fspec_core::dispatch with workUnitId='AUTH-001' and example='X'
    And I run the binary `fspec add-example AUTH-002 "Y"` against the same workspace shape
    Then both invocations call commands::add_example::run with the same JSON-marshalled args
    And the resulting spec/work-units.json contains exactly one new example per call
