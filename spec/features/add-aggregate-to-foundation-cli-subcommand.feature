@done
@RPC-166
@rust
@cli
@mutation
Feature: fspec add-aggregate-to-foundation CLI subcommand (Rust port)

  """
  Clap derive subcommand `add-aggregate-to-foundation` mirrors the TS Commander.js
  registration at src/commands/add-aggregate-to-foundation.ts:141-161 — two positional
  arguments `<context-name>` and `<aggregate-name>` plus an optional `-d, --description
  <text>` flag. The CLI bridge at codelet/fspec/src/add_aggregate_to_foundation.rs marshals
  the clap args into a JSON object and delegates to
  codelet_fspec_core::commands::add_aggregate_to_foundation::run; no validation, lookup, or
  file IO logic is duplicated in the bridge.

  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed
  with 'Error:' (parity with the chalk-red TS error path at
  src/commands/add-aggregate-to-foundation.ts:126-132).

  The `fspec add-aggregate-to-foundation --help` output is byte-for-byte identical to
  `node dist/index.js add-aggregate-to-foundation --help` piped to non-TTY (captured fixture
  at codelet/fspec/tests/fixtures/help/add-aggregate-to-foundation.txt).
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want an `add-aggregate-to-foundation` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes add-aggregate-to-foundation with positional args and description flag in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-aggregate-to-foundation --help`
    Then the command exits 0
    And stdout describes the add-aggregate-to-foundation subcommand
    And stdout mentions the `<context-name>` argument
    And stdout mentions the `<aggregate-name>` argument
    And stdout mentions the `--description` option

  Scenario: CLI adds an aggregate and prints the success message on stdout
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I run `./codelet/target/release/fspec add-aggregate-to-foundation Sales Order`
    Then the command exits 0
    And stdout contains the line '✓ Added aggregate "Order" to "Sales" bounded context'
    And spec/foundation.json eventStorm.items contains one aggregate item with text='Order'

  Scenario: CLI persists the optional description via the -d flag
    Given spec/foundation.json contains a bounded_context item 'Billing' with id=0 in eventStorm.items
    When I run `./codelet/target/release/fspec add-aggregate-to-foundation Billing Invoice -d "Billing root"`
    Then the command exits 0
    And the aggregate 'Invoice' has description='Billing root'

  Scenario: CLI rejects an unknown bounded context with exit 1 and stderr Error prefix
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I run `./codelet/target/release/fspec add-aggregate-to-foundation Unknown Order`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Bounded context 'Unknown' not found"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    When I dispatch add-aggregate-to-foundation via fspec_core::dispatch::dispatch_command with contextName='Sales' aggregateName='ViaDispatcher'
    Then the dispatcher writes spec/foundation.json
    And running `./codelet/target/release/fspec add-aggregate-to-foundation Sales ViaCli` afterwards exits 0
    And spec/foundation.json eventStorm.items contains two aggregate items
    And the CLI bridge module codelet/fspec/src/add_aggregate_to_foundation.rs contains NO inline bounded-context lookup, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling

  Scenario: add-aggregate-to-foundation --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-aggregate-to-foundation --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-aggregate-to-foundation.txt
