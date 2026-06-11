@done
@RPC-267
@rust
@cli
@mutation
Feature: fspec remove-architecture-note CLI subcommand (Rust port)

  """
  Clap derive subcommand `remove-architecture-note` exposes the same surface as the TS Commander.js registration at src/commands/remove-architecture-note.ts:88-107 — two positional arguments `<workUnitId>` and `<index>` (integer). The bridge module at codelet/fspec/src/remove_architecture_note.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::remove_architecture_note::run; no soft-delete or rendering logic is duplicated.
  Exit codes: 0 on success (including the idempotent already-deleted path), 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:'.
  The `fspec remove-architecture-note --help` output is byte-for-byte identical to `node dist/index.js remove-architecture-note --help` — captured as codelet/fspec/tests/fixtures/help/remove-architecture-note.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `remove-architecture-note` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes remove-architecture-note with positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-architecture-note --help`
    Then the command exits 0
    And stdout describes the remove-architecture-note subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<index>` argument
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI soft-deletes an architecture note and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNotes ids 0 and 1
    When I run `./codelet/target/release/fspec remove-architecture-note AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Architecture note removed successfully'
    And spec/work-units.json work unit 'AUTH-001' architectureNotes[0] has deleted=true

  Scenario: CLI reports the idempotent message when re-deleting
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 already deleted
    When I run `./codelet/target/release/fspec remove-architecture-note AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Architecture note removed successfully'
    And stdout contains the line '  Item ID 0 already deleted'

  Scenario: CLI rejects an unknown work unit with exit 1 and stderr Error prefix
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I run `./codelet/target/release/fspec remove-architecture-note MISSING-001 0`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0
    When I dispatch remove-architecture-note via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    Then the dispatcher writes spec/work-units.json
    And the CLI bridge module codelet/fspec/src/remove_architecture_note.rs contains NO inline soft-delete, ID-lookup, or file-write logic — its only computation is JSON arg marshalling

  Scenario: remove-architecture-note --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec remove-architecture-note --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-architecture-note.txt
    And stdout starts with a blank line followed by 'REMOVE-ARCHITECTURE-NOTE'
