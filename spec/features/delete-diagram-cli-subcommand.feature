@done
@RPC-216
@rust
@cli
@mutation
Feature: fspec delete-diagram CLI subcommand (Rust port)

  """
  Clap derive subcommand `delete-diagram` mirrors the TS Commander.js registration at
  src/commands/delete-diagram.ts:104-110 — two required positional arguments
  `<section>` and `<title>`. The CLI bridge at codelet/fspec/src/delete_diagram.rs
  marshals the clap args into a JSON object and delegates to
  codelet_fspec_core::commands::delete_diagram::run; no validation, file IO, or
  rendering logic is duplicated in the bridge.

  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr
  prefixed with 'Error:' (parity with the chalk-red TS error path at
  src/commands/delete-diagram.ts:88-100).

  The `fspec delete-diagram --help` output is byte-for-byte identical to
  `node dist/index.js delete-diagram --help` piped to non-TTY (captured fixture at
  codelet/fspec/tests/fixtures/help/delete-diagram.txt).
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `delete-diagram` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes delete-diagram with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec delete-diagram --help`
    Then the command exits 0
    And stdout describes the delete-diagram subcommand
    And stdout mentions the `<section>` argument
    And stdout mentions the `<title>` argument

  Scenario: CLI removes a diagram by title and prints the success block on stdout
    Given spec/foundation.json contains a diagram titled 'Component Flow'
    When I run `./codelet/target/release/fspec delete-diagram Architecture "Component Flow"`
    Then the command exits 0
    And stdout contains the line "✓ Deleted diagram 'Component Flow' from section 'Architecture'"
    And stdout contains the substring '  Updated: spec/foundation.json'
    And stdout contains the substring '  Regenerated: spec/FOUNDATION.md'
    And spec/foundation.json architectureDiagrams is empty

  Scenario: CLI fails with exit 1 when foundation.json is missing
    Given an empty project root directory with no spec/ subdirectory
    When I run `./codelet/target/release/fspec delete-diagram Architecture "X"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'foundation.json not found: spec/foundation.json'

  Scenario: CLI fails with exit 1 when title is not found
    Given spec/foundation.json contains a diagram titled 'Existing'
    When I run `./codelet/target/release/fspec delete-diagram Architecture "Missing"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Diagram 'Missing' not found in section 'Architecture'"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json contains diagrams titled 'A' and 'B'
    When I dispatch delete-diagram via fspec_core::dispatch::dispatch_command with section='Architecture' title='A'
    Then the dispatcher writes spec/foundation.json
    And running `./codelet/target/release/fspec delete-diagram Architecture "B"` afterwards exits 0
    And spec/foundation.json architectureDiagrams is empty
    And the CLI bridge module codelet/fspec/src/delete_diagram.rs contains NO inline file IO, JSON-parse, or splice logic — its only computation is JSON arg marshalling

  Scenario: delete-diagram --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec delete-diagram --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/delete-diagram.txt
