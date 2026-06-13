@done
@querying
@cli
@rust
@RPC-280
Feature: Remove schedule CLI subcommand

  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant (Mode::RemoveSchedule) with a single positional <name> argument, per RPC-003 §7/§11. The intercept_ts_help() pre-clap routine routes `remove-schedule --help` to codelet/fspec-core/src/help/configs/remove_schedule.rs which mirrors src/commands/remove-schedule-help.ts byte-for-byte under formatCommandHelp. The bridge codelet/fspec/src/remove_schedule.rs marshals the name into JSON and delegates to fspec_core::commands::remove_schedule::run — no mutation or IO.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec remove-schedule --help` and `fspec remove-schedule <name>` directly from a shell with the same rich formatCommandHelp output and behaviour offered by the TypeScript CLI
    So that I can delete schedules and discover the usage, examples, and related commands with byte-for-byte parity


  Scenario: remove-schedule --help is byte-for-byte identical to the TS reference output
    Given the fspec Rust binary has been compiled
    When I run `fspec remove-schedule --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/remove-schedule.txt
    Then stdout describes the single positional <name> argument and advertises no flags


  Scenario: CLI removes a schedule and delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/schedules.json contains a schedule named 'nightly-review'
    When I run `fspec remove-schedule nightly-review` from a shell against that project root
    Then the command exits 0
    Then spec/schedules.json contains no schedule named 'nightly-review'
    Then the CLI bridge module codelet/fspec/src/remove_schedule.rs contains NO schedule-mutation or file-writing logic beyond JSON arg marshalling

