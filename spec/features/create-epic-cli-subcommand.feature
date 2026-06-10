@done
@RPC-211
@rust
@cli
@mutation
Feature: fspec create-epic CLI subcommand (Rust port)

  """
  Clap derive subcommand `create-epic` exposes the same surface as the TS Commander.js registration at src/commands/create-epic.ts:115-127 — two positional arguments `<epicId>` and `<title>` plus an optional `-d, --description <description>` flag. The bridge module at codelet/fspec/src/create_epic.rs marshals the clap args into a JSON object and delegates to codelet_fspec_core::commands::create_epic::run; no validation or rendering logic is duplicated.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:' (parity with the chalk-red TS error path at src/commands/create-epic.ts:107-109).
  The `fspec create-epic --help` output is byte-for-byte identical to `node dist/index.js create-epic --help` (TS reference) — captured as codelet/fspec/tests/fixtures/help/create-epic.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `create-epic` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes create-epic with positional args and a --description flag in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-epic --help`
    Then the command exits 0
    And stdout describes the create-epic subcommand
    And stdout mentions the `<epicId>` argument
    And stdout mentions the `<title>` argument
    And stdout advertises the `--description` flag (or its `-d` short form)
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI creates a minimal epic and prints the success block
    Given an empty working directory with no spec/ subdirectory
    When I run `./codelet/target/release/fspec create-epic auth Authentication`
    Then the command exits 0
    And stdout contains the line '✓ Created epic auth'
    And stdout contains the line '  Title: Authentication'
    And stdout does NOT contain the substring 'Description:'
    And the file spec/epics.json exists

  Scenario: CLI creates an epic with -d description and includes the Description line
    Given an empty working directory with no spec/ subdirectory
    When I run `./codelet/target/release/fspec create-epic auth Authentication -d "Login features"`
    Then the command exits 0
    And stdout contains the line '✓ Created epic auth'
    And stdout contains the line '  Title: Authentication'
    And stdout contains the line '  Description: Login features'

  Scenario: CLI rejects an invalid epicId with exit 1 and stderr Error prefix
    Given an empty working directory with no spec/ subdirectory
    When I run `./codelet/target/release/fspec create-epic INVALID Authentication`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'lowercase-with-hyphens format'
    And the file spec/epics.json does NOT exist

  Scenario: CLI rejects creating an epic that already exists with exit 1
    Given spec/epics.json contains epic 'auth' with title='Old Title'
    When I run `./codelet/target/release/fspec create-epic auth NewTitle`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Epic auth already exists'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given an empty working directory with no spec/ subdirectory
    When I dispatch create-epic via fspec_core::dispatch::dispatch_command with epicId='auth' title='Authentication'
    Then the dispatcher writes spec/epics.json
    And running `./codelet/target/release/fspec create-epic dash Dashboard` afterwards exits 0
    And spec/epics.json now contains both 'auth' and 'dash' epics
    And the CLI bridge module codelet/fspec/src/create_epic.rs contains NO inline epic-id validation, duplicate-check, or file-write logic — its only computation is JSON arg marshalling

  Scenario: create-epic --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec create-epic --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-epic.txt
    And stdout starts with a blank line followed by 'CREATE-EPIC'
