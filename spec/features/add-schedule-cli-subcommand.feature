@done
@querying
@cli
@rust
@RPC-191
Feature: Add schedule CLI subcommand
  """
  CLI subcommand is wired into codelet/fspec/src/main.rs's Mode enum as a clap v4 derive variant (Mode::AddSchedule) with -n/--name, -c/--cron, -z/--timezone, -t/--type, -r/--role, -p/--prompt, --command, -o/--overlap flags, per RPC-003 §7/§11. The intercept_ts_help() pre-clap routine routes `add-schedule --help` to codelet/fspec-core/src/help/configs/add_schedule.rs which mirrors src/commands/add-schedule-help.ts byte-for-byte under formatCommandHelp. The bridge codelet/fspec/src/add_schedule.rs marshals CliArgs to JSON and delegates to fspec_core::commands::add_schedule::run — no validation or IO.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec add-schedule --help` and `fspec add-schedule -n <name> -c <cron> -z <tz> -t <type> ...` directly from a shell with the same rich formatCommandHelp output and behaviour offered by the TypeScript CLI
    So that I can register schedules and discover the flags, examples, and related commands with byte-for-byte parity

  Scenario: add-schedule --help is byte-for-byte identical to the TS reference output
    Given the fspec Rust binary has been compiled
    When I run `fspec add-schedule --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the TS reference output at codelet/fspec/tests/fixtures/help/add-schedule.txt
    Then stdout advertises the -n/--name, -c/--cron, -z/--timezone, -t/--type, -r/--role, -p/--prompt, --command, and -o/--overlap flags

  Scenario: CLI registers an agent schedule and delegates to the same fspec_core function as the dispatcher
    Given an empty project root directory
    When I run `fspec add-schedule -n nightly-review -c "0 2 * * *" -z UTC -t agent -r "Security reviewer" -p "Review src/"` from a shell against that project root
    Then the command exits 0
    Then spec/schedules.json contains a schedule named 'nightly-review' with jobType='agent'
    Then the CLI bridge module codelet/fspec/src/add_schedule.rs contains NO validation, schedule-construction, or file-writing logic beyond JSON arg marshalling
