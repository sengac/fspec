@done
@querying
@cli
@rust
@RPC-250
Feature: List schedules CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant with --json flag, per RPC-003 §7/§11. The intercept_ts_help() pre-clap routine in main.rs dispatches `list-schedules --help` to codelet/fspec-core/src/help/configs/list_schedules.rs which mirrors src/commands/list-schedules-help.ts byte-for-byte under formatCommandHelp.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec list-schedules --help` directly from a shell with the same rich formatCommandHelp output offered by the TypeScript CLI
    So that I can discover the --json flag, examples, and related commands from a script or terminal with byte-for-byte parity


  Scenario: list-schedules --help is byte-for-byte identical to TS reference output
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec list-schedules --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/list-schedules.txt
    And stdout starts with a blank line followed by 'LIST-SCHEDULES'
    And stdout contains the section header 'OPTIONS' followed by '  --json'
    And stdout contains the line 'fspec add-schedule - Create a new schedule'
    And stdout contains the section header 'NOTES' listing exactly 5 notes


  Scenario: CLI surface accepts only the --json flag
    Given the fspec Rust binary has been compiled with the list-schedules subcommand registered
    When I run `fspec list-schedules --help` from a shell
    Then the command exits 0
    And stdout describes the list-schedules subcommand
    And stdout advertises the --json flag
    And stdout does NOT advertise the substrings '--status', '--prefix', '--epic', '--format', '--category', or '--workspace'


  Scenario: CLI delegates to the same fspec_core function as dispatcher
    Given a project root whose spec/schedules.json contains one shell schedule
    When I dispatch list-schedules through fspec_core::dispatch::dispatch_command with format='json' AND I also invoke `fspec list-schedules --json` from a shell against the same project root
    Then both call sites return the identical pretty-printed JSON payload byte-for-byte
    And the CLI bridge module codelet/fspec/src/list_schedules.rs contains NO inline schedule-aggregation, filter, or rendering logic
    And the bridge module's only computation is the boolean-to-format-key JSON arg marshalling

