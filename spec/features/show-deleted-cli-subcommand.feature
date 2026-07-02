@done
@RPC-301
@rust
@querying
@cli
Feature: Show deleted CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_deleted::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes ONE positional argument <workUnitId> and NO flags — mirroring the TypeScript Commander.js registration at src/commands/show-deleted.ts:73-76 which only declares `.command('show-deleted').argument('<workUnitId>')` with no `.option(...)` calls. This is intentional: --status / --workspace / --format are all out of scope for RPC-301.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec show-deleted <workUnitId>` directly from a shell with the same positional-only surface offered by the TypeScript Commander.js CLI
    So that I can audit soft-deleted rules, examples, questions, and architecture notes from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-deleted as a subcommand with positional workUnitId and no flags
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-deleted --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the show-deleted subcommand
    Then stdout mentions the workUnitId positional argument
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI prints sentinel when the work unit exists but has no deleted items
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    Given spec/work-units.json contains AUTH-001 with no rules, examples, questions, or architectureNotes
    When I run `./codelet/target/release/fspec show-deleted AUTH-001` from that directory
    Then the command exits 0
    Then stdout contains the substring 'No deleted items found'

  Scenario: CLI text output renders header and item lines for the populated case
    Given spec/work-units.json contains AUTH-001 with one deleted rule (id=0, text='Old rule', deletedAt='2025-01-31T12:00:00.000Z') and one deleted example (id=1, text='Obsolete example', deletedAt='2025-02-01T08:00:00.000Z')
    When I run `./codelet/target/release/fspec show-deleted AUTH-001`
    Then the command exits 0
    Then stdout contains the substring 'Deleted items in AUTH-001 (2 total):'
    Then stdout contains the exact line '  [0] Old rule (deleted: 2025-01-31T12:00:00.000Z)'
    Then stdout contains the exact line '  [1] Obsolete example (deleted: 2025-02-01T08:00:00.000Z)'

  Scenario: CLI exits 1 and writes to stderr when the work unit does not exist
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-deleted UNKNOWN-999`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring "Work unit 'UNKNOWN-999' does not exist"

  Scenario: CLI auto-creates work-units.json before checking for the requested work unit
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-deleted AUTH-001`
    Then the command exits with code 1
    Then spec/work-units.json was created in the directory

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-deleted registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and show-deleted as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains AUTH-001 with one deleted rule (id=5, text='Shared', deletedAt='2025-03-01T00:00:00.000Z')
    When I dispatch show-deleted through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher's DispatchResult.data shows totalDeleted=1 with text='Shared'
    Then the CLI text output `fspec show-deleted AUTH-001` shows the exact line '  [5] Shared (deleted: 2025-03-01T00:00:00.000Z)' against the same on-disk state
    Then the CLI bridge module codelet/fspec/src/show_deleted.rs contains NO inline deleted-item collection, filter, or rendering logic — its only computation is JSON arg marshalling

  Scenario: show-deleted --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-deleted --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-deleted.txt
    And stdout starts with a blank line followed by 'SHOW-DELETED'
