@done
@RPC-287
Feature: fspec restore-architecture-note CLI subcommand (Rust port)

  """
  Clap derive subcommand `restore-architecture-note` exposes the same surface as the TS Commander.js registration at src/commands/restore-architecture-note.ts:88-107 — two positional arguments `<workUnitId>` and `<index>`, no flags wired into the action (the help-doc advertises `--ids` but the TS CLI does not implement it; we mirror Commander's actual surface — positional-only).
  The bridge module at codelet/fspec/src/restore_architecture_note.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::restore_architecture_note::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:'.
  The `fspec restore-architecture-note --help` output is byte-for-byte identical to `node dist/index.js restore-architecture-note --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/restore-architecture-note.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `restore-architecture-note` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands


  Scenario: Clap exposes restore-architecture-note with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec restore-architecture-note --help`
    Then the command exits 0
    And stdout describes the restore-architecture-note subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout mentions the `<index>` argument
    And stdout does NOT advertise a `--workspace` global flag


  Scenario: CLI restores an architecture note and prints the success line
    Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 text 'Note A' marked deleted
    When I run `./codelet/target/release/fspec restore-architecture-note AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Architecture note restored successfully'


  Scenario: CLI prints idempotent message when already active
    Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 text 'Note A' deleted=false
    When I run `./codelet/target/release/fspec restore-architecture-note AUTH-001 0`
    Then the command exits 0
    And stdout contains the line '✓ Architecture note restored successfully'
    And stdout contains the substring 'Item ID 0 already active'


  Scenario: CLI rejects unknown note ID with exit 1 and stderr Error prefix
    Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted
    When I run `./codelet/target/release/fspec restore-architecture-note AUTH-001 5`
    Then the command exits with code 1
    And stderr contains the substring 'Architecture note with ID 5 not found'


  Scenario: CLI rejects unknown work unit with exit 1 and stderr Error prefix
    Given spec/work-units.json contains no work unit 'AUTH-999'
    When I run `./codelet/target/release/fspec restore-architecture-note AUTH-999 0`
    Then the command exits with code 1
    And stderr contains the substring 'Work unit'


  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit 'AUTH-001' with two architectureNotes id=0 and id=1 both marked deleted
    When I dispatch restore-architecture-note via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    Then the dispatcher mutates spec/work-units.json
    And running `./codelet/target/release/fspec restore-architecture-note AUTH-001 1` afterwards exits 0
    And spec/work-units.json on disk shows both architectureNotes with deleted=false
    And the CLI bridge module codelet/fspec/src/restore_architecture_note.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling


  Scenario: restore-architecture-note --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec restore-architecture-note --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/restore-architecture-note.txt
    And stdout starts with a blank line followed by 'RESTORE-ARCHITECTURE-NOTE'
