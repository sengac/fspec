@done
@RPC-248
@rust
@querying
@cli
Feature: List prefixes CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_prefixes::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  The subcommand exposes NO flags — mirroring the TypeScript Commander.js registration at src/commands/list-prefixes.ts:101-104 which only declares `.command('list-prefixes').description('List all prefixes')` with no `.option(...)` calls. This is intentional: --status / --prefix / --epic / --format / --workspace are all out of scope for RPC-248.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-prefixes` directly from a shell with the same flag-less surface offered by the TypeScript Commander.js CLI
    So that I can browse registered prefixes and their completion progress from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-prefixes as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-prefixes --help` from a shell
    Then the command exits 0
    Then stdout contains clap-generated help describing the list-prefixes subcommand
    Then stdout does NOT contain the substring '--status'
    Then stdout does NOT contain the substring '--prefix'
    Then stdout does NOT contain the substring '--epic'
    Then stdout does NOT contain the substring '--format'
    Then stdout does NOT contain the substring '--workspace'

  Scenario: CLI against empty directory prints sentinel and does not auto-create files
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec list-prefixes` from that directory
    Then the command exits 0
    Then stdout contains the substring 'No prefixes found'
    Then spec/prefixes.json was NOT created in the directory
    Then spec/work-units.json was NOT created in the directory

  Scenario: CLI text output renders prefix progress for the populated case
    Given spec/prefixes.json contains AUTH (description 'Auth features') and DASH (description 'Dashboard') in that order
    Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), DASH-001 (done), DASH-002 (done)
    When I run `./codelet/target/release/fspec list-prefixes`
    Then the command exits 0
    Then stdout contains the substring 'Prefixes (2)'
    Then stdout contains the substring 'AUTH'
    Then stdout contains the substring '  Auth features'
    Then stdout contains the exact line '  Work Units: 1/2 (50%)'
    Then stdout contains the substring 'DASH'
    Then stdout contains the exact line '  Work Units: 2/2 (100%)'

  Scenario: CLI exits 1 and writes to stderr when prefixes.json is malformed
    Given spec/prefixes.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec list-prefixes`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse prefixes.json'

  Scenario: CLI exits 0 when work-units.json is malformed (work-unit read errors are silently swallowed)
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I run `./codelet/target/release/fspec list-prefixes`
    Then the command exits 0
    Then stdout contains the substring 'AUTH'
    Then stdout does NOT contain the substring 'Work Units:'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-prefixes registered as a clap subcommand alongside daemon, client, status, and list-work-units
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists daemon, client, status, list-work-units, and list-prefixes as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
    Given a project root whose spec/prefixes.json contains AUTH (description 'Auth features') and spec/work-units.json contains AUTH-001 (done) and AUTH-002 (backlog)
    When I dispatch list-prefixes through fspec_core::dispatch::dispatch_command with format='json'
    Then the dispatcher's DispatchResult.data shows AUTH at 1/2 (50%) and the CLI text output (`fspec list-prefixes`) shows the exact line '  Work Units: 1/2 (50%)' against the same on-disk state
    Then the CLI bridge module codelet/fspec/src/list_prefixes.rs contains NO inline prefix-aggregation, filter, or rendering logic — its only computation is JSON arg marshalling
