@done
@querying
@cli
@RPC-303
Feature: show-event-storm clap subcommand on the standalone fspec Rust binary

  """
  CLI surface for the `show-event-storm` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note [7] on RPC-253, reused for RPC-303):
    - Shell argv         → clap → codelet/fspec/src/show_event_storm.rs → fspec_core::commands::show_event_storm::run
    - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::show_event_storm::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes exactly one required positional argument <work-unit-id> and no flags — matching TS Commander.js registration at src/commands/show-event-storm.ts:107-115.
  Stdout receives the pretty-printed JSON array of active eventStorm items on success.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/show-event-storm.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a show-event-storm clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that Event Storm display logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes show-event-storm as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-event-storm --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'show-event-storm'
    Then stdout advertises the required positional <work-unit-id> argument
    Then stdout does NOT contain the substring '--format'

  Scenario: CLI against empty workspace exits 1 with Work unit not found
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec show-event-storm AUTH-001` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Work unit AUTH-001 not found'

  Scenario: CLI exits 1 with no Event Storm data error when the unit has no eventStorm field
    Given spec/work-units.json contains AUTH-001 with no eventStorm field
    When I run `./codelet/target/release/fspec show-event-storm AUTH-001` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Work unit AUTH-001 has no Event Storm data'

  Scenario: CLI prints the JSON array of active items on success
    Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), event(id=1, deleted=true), command(id=2, deleted=false)]
    When I run `./codelet/target/release/fspec show-event-storm AUTH-001` from that directory
    Then the command exits 0
    Then stdout parses as a JSON array of length 2
    Then the parsed array[0] has id=0
    Then the parsed array[1] has id=2

  Scenario: CLI exits 1 when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec show-event-storm AUTH-001`
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false)]
    When I dispatch show-event-storm through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    Then the DispatchResult.data parses as a JSON array of length 1
    Then the CLI bridge module codelet/fspec/src/show_event_storm.rs contains NO inline filter or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: show-event-storm --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-event-storm --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-event-storm.txt
    Then stdout starts with a blank line followed by 'SHOW-EVENT-STORM'
