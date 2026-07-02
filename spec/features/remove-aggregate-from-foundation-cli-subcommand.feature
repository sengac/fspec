@done
@RPC-266
@rust
@cli
@mutation
Feature: fspec remove-aggregate-from-foundation CLI subcommand (Rust port)
  """
  Clap derive subcommand `remove-aggregate-from-foundation` mirrors the TS Commander.js
  registration at src/commands/remove-aggregate-from-foundation.ts:134-153 — two positional
  arguments `<context-name>` and `<aggregate-name>` (no options). The CLI bridge at
  codelet/fspec/src/remove_aggregate_from_foundation.rs marshals the clap args into a JSON
  object and delegates to
  codelet_fspec_core::commands::remove_aggregate_from_foundation::run; no validation, lookup,
  or file IO logic is duplicated in the bridge.

  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed
  with 'Error:' (parity with the chalk-red TS error path at
  src/commands/remove-aggregate-from-foundation.ts:119-125).

  The `fspec remove-aggregate-from-foundation --help` output is byte-for-byte identical to
  `node dist/index.js remove-aggregate-from-foundation --help` piped to non-TTY (captured
  fixture at codelet/fspec/tests/fixtures/help/remove-aggregate-from-foundation.txt).
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `remove-aggregate-from-foundation` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes remove-aggregate-from-foundation with positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-aggregate-from-foundation --help`
    Then the command exits 0
    And stdout describes the remove-aggregate-from-foundation subcommand
    And stdout mentions the `<context-name>` argument
    And stdout mentions the `<aggregate-name>` argument

  Scenario: CLI soft-deletes an aggregate and prints the success message on stdout
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    When I run `./codelet/target/release/fspec remove-aggregate-from-foundation Sales Order`
    Then the command exits 0
    And stdout contains the line '✓ Removed aggregate "Order" from "Sales" bounded context'
    And the aggregate 'Order' in eventStorm.items has deleted=true

  Scenario: CLI rejects an unknown aggregate with exit 1 and stderr Error prefix
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    When I run `./codelet/target/release/fspec remove-aggregate-from-foundation Sales Ghost`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Aggregate 'Ghost' not found in bounded context 'Sales'"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and aggregates 'Order' and 'Shipment' linked to it
    When I dispatch remove-aggregate-from-foundation via fspec_core::dispatch::dispatch_command with contextName='Sales' aggregateName='Order'
    Then the dispatcher writes spec/foundation.json
    And running `./codelet/target/release/fspec remove-aggregate-from-foundation Sales Shipment` afterwards exits 0
    And both the 'Order' and 'Shipment' aggregates have deleted=true
    And the CLI bridge module codelet/fspec/src/remove_aggregate_from_foundation.rs contains NO inline bounded-context lookup, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling

  Scenario: remove-aggregate-from-foundation --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-aggregate-from-foundation --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-aggregate-from-foundation.txt
