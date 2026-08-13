@done
@RPC-263
@cli
@querying
@rust
Feature: query-work-units CLI subcommand on the standalone fspec Rust binary
  """
  This feature describes the shell-facing surface for `fspec query-work-units`
  exposed via clap on the standalone Rust binary. Per the two-front-doors
  invariant (RPC-003 §7/§11), the CLI bridge (rust/fspec/src/query_work_units.rs)
  delegates to fspec_core::commands::query_work_units::run rather than duplicating
  filter or rendering logic.

  Mirrors the TS Commander.js registration exactly: --status, --prefix, --epic,
  --type, --tag, --format (default 'text'). Other function-level options
  (sort/order/output/hasQuestions/questionsFor/showCycleTime/workUnitId/json)
  are NOT exposed at the CLI surface — they are dispatcher-only.

  TS quirk preserved: when --format=json the CLI prints JSON to stdout; for any
  other format the CLI prints NOTHING to stdout (the TS Commander action only
  calls output.log when options.format === 'json').
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec query-work-units` from a shell with the same six flags as the TypeScript CLI
    So that scripts and humans get byte-for-byte identical results without depending on Node.js

  Scenario: Standalone fspec binary exposes query-work-units as a clap subcommand
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-work-units --help` from a shell
    Then the command exits 0
    Then stdout lists the flags --status, --prefix, --epic, --type, --tag, and --format

  Scenario: CLI query-work-units --format=json prints parseable JSON to stdout
    Given spec/work-units.json contains AUTH-001 (implementing) and AUTH-002 (backlog)
    When I run `./rust/target/release/fspec query-work-units --status=implementing --format=json`
    Then the command exits 0
    Then stdout is a parseable JSON object whose workUnits array contains only AUTH-001
    Then the parsed JSON object contains a top-level `format` field equal to 'json'

  Scenario: CLI query-work-units --format=text prints NOTHING to stdout per TS quirk
    Given spec/work-units.json contains AUTH-001 (implementing)
    When I run `./rust/target/release/fspec query-work-units --status=implementing --format=text`
    Then the command exits 0
    Then stdout is empty (the TS Commander action does NOT log for non-json formats)

  Scenario: CLI query-work-units --tag filter matches dispatcher behavior
    Given spec/work-units.json contains AUTH-001 (tags ['@cli']) and AUTH-002 (tags ['@high'])
    When I run `./rust/target/release/fspec query-work-units --tag=@cli --format=json`
    Then the command exits 0
    Then stdout's workUnits array contains only AUTH-001

  Scenario: CLI query-work-units exits 1 and writes to stderr when spec/work-units.json is missing
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-work-units` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Failed to query work units:'
    Then spec/work-units.json is NOT auto-created in the directory

  Scenario: CLI query-work-units exits 1 and writes to stderr when spec/work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./rust/target/release/fspec query-work-units` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Failed to query work units:'

  Scenario: Subcommand help excludes the global workspace flag
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-work-units --help` from a shell
    Then the command exits 0
    Then stdout does NOT contain the substring '--workspace'

  Scenario: query-work-units --help matches TS formatCommandHelp reference fixture
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-work-units --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-work-units.txt
    Then stdout starts with a blank line followed by 'QUERY-WORK-UNITS'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has query-work-units registered as a clap subcommand alongside daemon, client, status, and list-work-units
    When I run `./rust/target/release/fspec --help`
    Then the command exits 0
    Then the help output lists query-work-units as an available subcommand
