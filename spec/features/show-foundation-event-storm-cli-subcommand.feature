@done
@event-storming
@cli
@RPC-306
Feature: show-foundation-event-storm CLI subcommand on the standalone fspec Rust binary

  """
  CLI subcommand wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003 §7/§11. The action arm delegates to fspec_core::commands::show_foundation_event_storm::run(args_json) so business logic is not duplicated between the LLM-facing dispatcher and the shell-facing CLI.

  show-foundation-event-storm exposes two optional flags: --type <type> and --context <name>. There are no positional arguments.

  Exit-code contract: 0 on success (JSON array to stdout); 1 when fspec_core::commands::show_foundation_event_storm::run returns FspecCoreError (foundation.json missing OR malformed) or the dispatcher envelope returns success=false. Error messages go to stderr prefixed with 'Error:'.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec show-foundation-event-storm` from a shell with the same optional --type and --context flags offered by the TypeScript Commander.js CLI
    So that I can audit foundation Event Storm artifacts from a script without going through the LLM tool-call dispatcher

  Scenario: Clap exposes show-foundation-event-storm as a subcommand and prints flag help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-foundation-event-storm --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'show-foundation-event-storm'
    And stdout contains the substring '--type'
    And stdout contains the substring '--context'

  Scenario: CLI against workspace with no foundation.json exits 1 with error
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./codelet/target/release/fspec show-foundation-event-storm` from that directory
    Then the command exits 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'foundation.json'

  Scenario: CLI prints empty array when foundation.json has no eventStorm
    Given a temp workspace contains spec/foundation.json without an eventStorm field
    When I run `./codelet/target/release/fspec show-foundation-event-storm` from that workspace
    Then the command exits 0
    And stdout parses as a JSON array with 0 elements

  Scenario: CLI prints all active items when no filters are supplied
    Given a temp workspace contains spec/foundation.json with three active eventStorm items and one item where deleted=true
    When I run `./codelet/target/release/fspec show-foundation-event-storm` from that workspace
    Then the command exits 0
    And stdout parses as a JSON array with 3 elements

  Scenario: CLI --type filter narrows to matching items
    Given a temp workspace contains spec/foundation.json with two aggregates, one bounded_context, and one event
    When I run `./codelet/target/release/fspec show-foundation-event-storm --type aggregate` from that workspace
    Then the command exits 0
    And stdout parses as a JSON array with 2 elements
    And every JSON element has type='aggregate'

  Scenario: CLI --context filter returns bounded context plus linked items
    Given a temp workspace contains spec/foundation.json with bounded_context id=1 text='Work Management' plus three items linked to it
    When I run `./codelet/target/release/fspec show-foundation-event-storm --context "Work Management"` from that workspace
    Then the command exits 0
    And stdout parses as a JSON array with 4 elements

  Scenario: CLI --context with unknown name prints empty array
    Given a temp workspace contains spec/foundation.json with a bounded_context text='Work Management' and items linked to it
    When I run `./codelet/target/release/fspec show-foundation-event-storm --context Nonexistent` from that workspace
    Then the command exits 0
    And stdout parses as a JSON array with 0 elements

  Scenario: show-foundation-event-storm --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec show-foundation-event-storm --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-foundation-event-storm.txt
    And stdout starts with a blank line followed by 'SHOW-FOUNDATION-EVENT-STORM'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has show-foundation-event-storm registered as a clap subcommand alongside daemon, client, status, and other ported subcommands
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists show-foundation-event-storm as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/foundation.json with one bounded_context 'Work Management' and two linked aggregates
    When I dispatch show-foundation-event-storm through fspec_core::dispatch::dispatch_command with context='Work Management' against that workspace
    And I run `./codelet/target/release/fspec show-foundation-event-storm --context "Work Management"` against the same workspace
    Then both invocations produce JSON arrays with 3 elements
    And the CLI bridge module codelet/fspec/src/show_foundation_event_storm.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
