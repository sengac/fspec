@done
@RPC-217
@rust
@cli
@mutation
Feature: fspec delete-epic CLI subcommand (Rust port)
  """
  Clap derive subcommand `delete-epic` exposes the same surface as the TS Commander.js registration at src/commands/delete-epic.ts:92-108 — a single positional `<epicId>` and an optional `--force` flag. The bridge module at rust/fspec/src/delete_epic.rs marshals these into a JSON object and delegates to codelet_fspec_core::commands::delete_epic::run; --force is parsed for parity but NOT forwarded into the dispatcher's logic because the TS implementation never reads its value (see src/commands/delete-epic.ts:98-99).
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with 'Error:' (parity with the chalk-red TS error path at src/commands/delete-epic.ts:104-107). The success line is '✓ Epic <id> deleted successfully'.
  The `fspec delete-epic --help` output is byte-for-byte identical to `node dist/index.js delete-epic --help` (TS reference) — captured as rust/fspec/tests/fixtures/help/delete-epic.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `delete-epic` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes delete-epic with a positional arg and --force flag in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec delete-epic --help`
    Then the command exits 0
    And stdout describes the delete-epic subcommand
    And stdout mentions the `<epicId>` argument
    And stdout advertises the `--force` flag
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI deletes an existing epic and prints the success line
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    When I run `./rust/target/release/fspec delete-epic auth`
    Then the command exits 0
    And stdout contains the line '✓ Epic auth deleted successfully'
    And the on-disk spec/epics.json no longer contains an 'auth' epic

  Scenario: CLI accepts --force without changing behaviour (TS impl ignores it)
    Given spec/epics.json contains epic 'auth' with title='Authentication'
    When I run `./rust/target/release/fspec delete-epic auth --force`
    Then the command exits 0
    And stdout contains the line '✓ Epic auth deleted successfully'

  Scenario: CLI exits 1 when the epic does not exist
    Given spec/epics.json contains epic 'dash' with title='Dashboard'
    When I run `./rust/target/release/fspec delete-epic missing`
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Epic missing not found'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/epics.json contains epics 'auth' and 'dash'
    When I dispatch delete-epic via fspec_core::dispatch::dispatch_command with epicId='auth'
    Then the dispatcher returns success=true
    And running `./rust/target/release/fspec delete-epic dash` afterwards exits 0
    And spec/epics.json contains neither 'auth' nor 'dash' epics
    And the CLI bridge module rust/fspec/src/delete_epic.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling

  Scenario: delete-epic --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec delete-epic --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/delete-epic.txt
    And stdout starts with a blank line followed by 'DELETE-EPIC'
