@done
@schedule-management
@rust
@cli
@RPC-254
Feature: Pause schedule CLI subcommand
  """
  The pause-schedule CLI subcommand is wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant taking a required positional <name>, per RPC-003 §7/§11. The CLI bridge rust/fspec/src/pause_schedule.rs is JSON marshalling only — it resolves project_root from CWD, marshals { name } to JSON, calls fspec_core::commands::pause_schedule::run, prints "✓ Schedule '<name>' paused successfully" on success (exit 0), or the failure message on stderr (exit 1).
  The intercept_ts_help() pre-clap routine in main.rs dispatches `pause-schedule --help` to rust/fspec-core/src/help/configs/pause_schedule.rs which mirrors src/commands/pause-schedule-help.ts byte-for-byte under formatCommandHelp.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec pause-schedule <name>` and `fspec pause-schedule --help` directly from a shell
    So that I can pause schedules from scripts and discover usage with byte-for-byte parity to the TypeScript CLI

  Scenario: CLI pauses an active schedule and prints a success message
    Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    When I run `fspec pause-schedule nightly-review` from a shell against that project root
    Then the command exits 0
    And stdout contains "✓ Schedule 'nightly-review' paused successfully"
    And spec/schedules.json now records the 'nightly-review' schedule with status 'paused'

  Scenario: CLI reports an error and exits 1 when the schedule does not exist
    Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    When I run `fspec pause-schedule ghost` from a shell against that project root
    Then the command exits 1
    And stderr contains "Schedule 'ghost' does not exist"
    And spec/schedules.json is unchanged

  Scenario: pause-schedule --help is byte-for-byte identical to TS reference output
    Given the fspec Rust binary has been compiled
    When I run `fspec pause-schedule --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the TS reference output at rust/fspec/tests/fixtures/help/pause-schedule.txt
    And stdout starts with a blank line followed by 'PAUSE-SCHEDULE'
    And stdout contains the section header 'ARGUMENTS' followed by '  <name> (required)'
    And stdout contains the line 'resume-schedule - Resume a paused schedule'

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/schedules.json contains an active schedule named 'nightly-review'
    When I dispatch pause-schedule through fspec_core::dispatch::dispatch_command with name='nightly-review' AND I separately invoke `fspec pause-schedule nightly-review` against an identical project root
    Then both call sites produce the identical status transition to 'paused' in spec/schedules.json
    And the CLI bridge module rust/fspec/src/pause_schedule.rs contains NO inline schedule-mutation, validation, or file-writing logic
    And the bridge module's only computation is marshalling the name argument into the JSON args shape
