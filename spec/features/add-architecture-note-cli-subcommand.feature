@done
@RPC-168
@rust
@cli
@mutation
Feature: fspec add-architecture-note CLI subcommand (Rust port)
  """
  Clap derive subcommand `add-architecture-note` exposes the same surface as the TS Commander.js registration at src/commands/add-architecture-note.ts:89-108 — two positional arguments `<workUnitId>` and `<note>`. The bridge module at codelet/fspec/src/add_architecture_note.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::add_architecture_note::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:' (parity with the chalk-red TS error path at src/commands/add-architecture-note.ts:102-107).
  The `fspec add-architecture-note --help` output is byte-for-byte identical to `node dist/index.js add-architecture-note --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/add-architecture-note.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want an `add-architecture-note` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes add-architecture-note with positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-architecture-note --help`
    Then the command exits 0
    And stdout describes the add-architecture-note subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<note>` argument
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI appends an architecture note and prints the success block
    Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    When I run `./codelet/target/release/fspec add-architecture-note AUTH-001 "Uses bcrypt"`
    Then the command exits 0
    And stdout contains the line '✓ Architecture note added successfully'
    And stdout contains the substring '<system-reminder>'
    And spec/work-units.json work unit 'AUTH-001' has one architectureNote with text='Uses bcrypt'

  Scenario: CLI rejects an unknown work unit with exit 1 and stderr Error prefix
    Given spec/work-units.json contains no work unit 'MISSING-001'
    When I run `./codelet/target/release/fspec add-architecture-note MISSING-001 "any note"`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Work unit 'MISSING-001' does not exist"

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001'
    When I dispatch add-architecture-note via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' note='via dispatcher'
    Then the dispatcher writes spec/work-units.json
    And running `./codelet/target/release/fspec add-architecture-note AUTH-001 "via cli"` afterwards exits 0
    And spec/work-units.json work unit 'AUTH-001' contains two architectureNotes
    And the CLI bridge module codelet/fspec/src/add_architecture_note.rs contains NO inline note-append, nextNoteId, or system-reminder rendering logic — its only computation is JSON arg marshalling

  Scenario: add-architecture-note --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-architecture-note --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-architecture-note.txt
    And stdout starts with a blank line followed by 'ADD-ARCHITECTURE-NOTE'
