@done
@RPC-242
@rust
@querying
@cli
Feature: List checkpoints CLI subcommand
  """
  CLI subcommand is wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_checkpoints::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes exactly ONE required positional argument <workUnitId> and NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-checkpoints.ts:83-88 which declares `.argument('<work-unit-id>')` and no `.option(...)` calls. This is intentional: --format / --workspace / --prefix are out of scope for RPC-242.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-checkpoints <work-unit-id>` directly from a shell with the same positional-only surface offered by the TypeScript Commander.js CLI
    So that I can browse manual and automatic checkpoints for a work unit from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-checkpoints as a subcommand with flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-checkpoints --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-checkpoints subcommand
    Then stdout describes a positional argument named work-unit-id or workUnitId
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--status'

  Scenario: list-checkpoints --help is byte-for-byte identical to the TS formatCommandHelp reference output
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-checkpoints --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/list-checkpoints.txt
    And stdout starts with a blank line followed by 'LIST-CHECKPOINTS'

  Scenario: Missing positional argument exits non-zero with clap's required-arg error
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-checkpoints` from a shell with NO positional argument
    Then the command exits with a non-zero code
    Then stderr contains the substring 'work-unit-id' or the substring 'workUnitId' or the substring 'WORK_UNIT_ID'

  Scenario: CLI against empty directory prints sentinel and does not auto-init a git repo
    Given an empty directory with no .git subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec list-checkpoints AUTH-001` from that directory
    Then the command exits 0
    Then stdout contains the substring 'No checkpoints found for AUTH-001'
    Then the directory does NOT contain a .git subdirectory after the call

  Scenario: CLI text output renders manual checkpoint progress for the populated case
    Given a git repository at the current working directory with a manual checkpoint 'baseline' for AUTH-001
    Given the checkpoint index file records timestamp '2026-06-01T10:00:00.000Z' for 'baseline'
    When I run `./rust/target/release/fspec list-checkpoints AUTH-001`
    Then the command exits 0
    Then stdout contains the substring 'Checkpoints for AUTH-001:'
    Then stdout contains the substring '📌  baseline (manual)'
    Then stdout contains the substring 'Created: 2026-06-01T10:00:00.000Z'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-checkpoints registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./rust/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-checkpoints as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose .git directory contains a manual checkpoint 'baseline' for AUTH-001 with index timestamp '2026-06-01T10:00:00.000Z'
    When I dispatch list-checkpoints through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher's DispatchResult.data shows checkpoint 'baseline' with timestamp '2026-06-01T10:00:00.000Z' and isAutomatic=false
    When I run `./rust/target/release/fspec list-checkpoints AUTH-001` against the same on-disk state
    Then stdout contains the substring '📌  baseline (manual)'
    Then the CLI bridge module rust/fspec/src/list_checkpoints.rs contains NO inline checkpoint-listing, classification or rendering logic — its only computation is JSON arg marshalling
