@done
@RPC-308
@cli
@querying
Feature: Show work unit CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_work_unit::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes ONE positional <workUnitId> argument and a -f/--format <format> option — mirroring the TypeScript Commander.js registration at src/commands/show-work-unit.ts:468-475.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run `fspec show-work-unit <workUnitId>` directly from a shell with the same positional surface offered by the TypeScript Commander.js CLI
    So that I can inspect a work unit (Example Mapping data, dependencies, linked features, reminders) from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-work-unit as a subcommand with positional workUnitId and a format option
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-work-unit --help` from a shell
    Then the command exits 0
    Then stdout contains help describing the show-work-unit subcommand
    Then stdout mentions the workUnitId positional argument
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI prints text-format dump when the work unit exists
    Given an empty directory is set as the current working directory
    Given spec/work-units.json contains AUTH-001 with title='Login', status='backlog', no rules
    When I run `./codelet/target/release/fspec show-work-unit AUTH-001` from that directory
    Then the command exits 0
    Then stdout contains the substring 'AUTH-001'
    Then stdout contains the substring 'Type: story'
    Then stdout contains the substring 'Status: backlog'
    Then stdout contains the substring 'Login'

  Scenario: CLI prints JSON payload when --format json is supplied
    Given an empty directory is set as the current working directory
    Given spec/work-units.json contains AUTH-001 with title='Login', status='backlog'
    When I run `./codelet/target/release/fspec show-work-unit AUTH-001 --format json`
    Then the command exits 0
    Then stdout parses as JSON with id='AUTH-001', title='Login', type='story', status='backlog'
    Then stdout uses 2-space indentation

  Scenario: CLI exits 1 and writes the canonical message to stderr when the work unit does not exist
    Given an empty directory is set as the current working directory
    Given spec/work-units.json contains AUTH-001 (any minimal shape) but NOT UNKNOWN-999
    When I run `./codelet/target/release/fspec show-work-unit UNKNOWN-999`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring "Work unit 'UNKNOWN-999' does not exist"

  Scenario: CLI exits 1 when spec/work-units.json is absent (no auto-create)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-work-unit AUTH-001`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then spec/work-units.json was NOT created in the directory

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-work-unit registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and show-deleted
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, show-deleted, and show-work-unit as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains AUTH-001 with title='Shared', status='backlog'
    When I dispatch show-work-unit through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    Then the dispatcher's DispatchResult.data shows id='AUTH-001' and title='Shared'
    Then the CLI text output `fspec show-work-unit AUTH-001` against the same on-disk state shows the substring 'AUTH-001' and the line 'Status: backlog'
    Then the CLI bridge module codelet/fspec/src/show_work_unit.rs contains NO inline projection, reminder generation, or feature-scan logic — its only computation is JSON arg marshalling

  Scenario: show-work-unit --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-work-unit --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-work-unit.txt
    And stdout starts with a blank line followed by 'SHOW-WORK-UNIT'
