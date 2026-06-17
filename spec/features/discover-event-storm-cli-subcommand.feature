@done
@querying
@cli
@RPC-225
Feature: discover-event-storm clap subcommand on the standalone fspec Rust binary

  """
  CLI surface for the `discover-event-storm` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (RPC-003 §7/§11, reused for RPC-225):
    - Shell argv         → clap → codelet/fspec/src/discover_event_storm.rs → fspec_core::commands::discover_event_storm::run
    - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::discover_event_storm::run
  Both call sites pass a JSON-encoded args shape and a project_root: &Path.
  The CLI surface resolves project_root from CWD (parity with TS process.cwd() default).
  The clap subcommand exposes exactly one required positional argument <work-unit-id> and no flags — matching TS Commander.js registration at src/commands/discover-event-storm.ts:83-90.
  Stdout receives the green confirmation line plus the Event Storm guidance system-reminder on success.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/discover-event-storm.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a discover-event-storm clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that Event Storm discovery guidance is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes discover-event-storm as a subcommand and prints flag-free --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec discover-event-storm --help` from a shell
    Then the command exits 0
    Then stdout is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/discover-event-storm.txt
    Then stdout advertises the required positional <work-unit-id> argument

  Scenario: CLI emits guidance for a work unit in specifying status and exits 0
    Given spec/work-units.json contains AUTH-001 in specifying status in the current working directory
    When I run `./codelet/target/release/fspec discover-event-storm AUTH-001` from that directory
    Then the command exits with code 0
    Then stdout contains '✓ Event Storm discovery session started for AUTH-001'
    Then stdout contains the substring 'EVENT STORM DISCOVERY - AUTH-001'

  Scenario: CLI against empty workspace exits 1 with missing-file error
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec discover-event-storm AUTH-001` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'spec/work-units.json not found. Run fspec init first.'

  Scenario: CLI exits 1 when the work unit is not in specifying status
    Given spec/work-units.json contains AUTH-001 in backlog status in the current working directory
    When I run `./codelet/target/release/fspec discover-event-storm AUTH-001` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'must be in specifying status (currently: backlog)'
