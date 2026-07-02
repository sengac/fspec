@done
@rust
@querying
@cli
@RPC-243
Feature: List epics CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_epics::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-epics.ts:141-146 which only declares `.command('list-epics').description('List all epics')` with no `.option(...)` calls. --status / --prefix / --epic / --format / --workspace are all out of scope for RPC-243.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-epics` directly from a shell with the same flag-less surface offered by the TypeScript Commander.js CLI
    So that I can browse registered epics and their completion progress from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-epics as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-epics --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-epics subcommand
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--epic'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against empty directory prints sentinel and does not auto-create files
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-epics` from that directory
    Then the command exits 0
    Then stdout contains the substring 'No epics found'
    Then spec/epics.json was NOT created in the directory
    Then spec/work-units.json was NOT created in the directory

  Scenario: CLI text output renders epic progress for the populated case
    Given spec/epics.json contains auth (title 'Authentication', description 'Login features') and dash (title 'Dashboard', description 'Dashboard features') in that order
    Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), DASH-001 (epic=dash, status=done), DASH-002 (epic=dash, status=done)
    When I run `./codelet/target/release/fspec list-epics`
    Then the command exits 0
    Then stdout contains the substring 'Epics (2)'
    Then stdout contains the substring 'auth'
    Then stdout contains the substring '  Authentication'
    Then stdout contains the substring '  Login features'
    Then stdout contains the exact line '  Work Units: 1/2 (50%)'
    Then stdout contains the substring 'dash'
    Then stdout contains the exact line '  Work Units: 2/2 (100%)'

  Scenario: CLI exits 1 and writes to stderr when epics.json is malformed
    Given spec/epics.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec list-epics`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse epics.json'

  Scenario: CLI exits 0 when work-units.json is malformed
    Given spec/epics.json contains auth (title 'Authentication')
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I run `./codelet/target/release/fspec list-epics`
    Then the command exits 0
    Then stdout contains the substring 'auth'
    Then stdout does NOT contain the substring 'Work Units:'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-epics registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-epics as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/epics.json contains auth (title 'Authentication') and spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=backlog)
    When I dispatch list-epics through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher's DispatchResult.data parses to an epics structure with auth at 1/2 (50%)
    Then the CLI text output reflects the same 1/2 (50%) progress
    Then the CLI bridge module codelet/fspec/src/list_epics.rs contains NO inline epic-aggregation, filter, or rendering logic — its only computation is JSON arg marshalling

  Scenario: list-epics --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-epics --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/list-epics.txt
    And stdout starts with a blank line followed by 'LIST-EPICS'
