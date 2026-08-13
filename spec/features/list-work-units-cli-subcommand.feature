@done
@RPC-253
@rust
@querying
@cli
Feature: List work units CLI subcommand
  """
  CLI subcommand is wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::list_work_units::run(args_json, &cwd) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-work-units` directly from a shell with the same flags supported by the TypeScript Commander.js CLI
    So that I can browse and filter work units from a script or terminal without going through the LLM tool-call dispatcher

  Scenario: Clap exposes list-work-units as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-work-units --help` from a shell
    Then the command exits 0 and prints TS-style help listing --status, --prefix, --epic flags

  Scenario: CLI against empty directory creates default files and prints sentinel
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec list-work-units` from that directory
    Then the command exits 0 and prints 'No work units found' to stdout
    Then spec/work-units.json and spec/prefixes.json are created in the directory

  Scenario: CLI emits 2-space indented JSON when --format=json is passed
    Given spec/work-units.json contains AUTH-001 (backlog, epic 'ux') and AUTH-002 (implementing)
    When I run `./rust/target/release/fspec list-work-units --format=json`
    Then the command exits 0 and stdout contains a parseable JSON object with a workUnits array of length 2
    Then the JSON includes id, title, status, and epic 'ux' for AUTH-001 in insertion order

  Scenario: CLI --status filter matches dispatcher behavior
    Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    When I run `./rust/target/release/fspec list-work-units --status=backlog --format=json`
    Then stdout contains a JSON workUnits array of length 2 with AUTH-001 and DASH-001 in that order
    Then the command exits 0

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./rust/target/release/fspec list-work-units`
    Then the command exits with code 1
    Then stderr contains the substring 'Failed to parse work-units.json'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has list-work-units registered as a clap subcommand alongside daemon, client, and status
    When I run `./rust/target/release/fspec --help`
    Then the help output lists daemon, client, status, and list-work-units as available subcommands
    Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI tolerates unknown work-unit type values for parity with the TypeScript runtime
    Given spec/work-units.json contains FEAT-001 with type 'feature' (a value outside story/task/bug) and AUTH-001 with no type field
    When I run `./rust/target/release/fspec list-work-units --type=story --format=json`
    Then the command exits 0 and stdout contains a JSON workUnits array of length 1 with AUTH-001
    Then stderr does NOT contain the substring 'unknown variant'

  Scenario: Subcommand help excludes the global workspace flag
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-work-units --help` from a shell
    Then the command exits 0
    Then stdout does NOT contain the substring '--workspace'

  Scenario: list-work-units --help is byte-for-byte identical to TS formatCommandHelp reference output
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec list-work-units --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/list-work-units.txt
    And stdout starts with a blank line followed by 'LIST-WORK-UNITS'
    And stdout contains the section header 'TYPICAL WORKFLOW'
