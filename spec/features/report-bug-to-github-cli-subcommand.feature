@done
@rust
@report-bug-to-github
@cli
@RPC-285
Feature: Report-bug-to-github CLI subcommand
  """
  CLI subcommand wired into rust/fspec/src/main.rs's Mode enum as a clap v4 derive variant per RPC-003
  §7/§11. The action arm delegates to codelet_fspec_core::commands::report_bug_to_github::run(args_json, &cwd)
  so the gather/format/URL business logic is not duplicated between the LLM-facing dispatcher and the
  shell-facing CLI.

  TS Commander.js registration (src/commands/report-bug-to-github.ts:359-413) declares:
  .command('report-bug-to-github')
  .option('--project-root <path>') .option('--bug-description <text>')
  .option('--expected-behavior <text>') .option('--actual-behavior <text>') .option('--interactive')
  The clap variant exposes the matching flags. The action handler prints 'Gathering system context...' before
  the report and, on success, prints the success banner; this port prints the constructed GitHub URL so the
  user can open it manually.

  ⚠️ SCOPE FLAG: actual automatic browser launch and real interactive stdin prompting are DEFERRED pending a
  supervisor scope decision — see report-bug-to-github-rust-port.feature docstring. This feature specifies the
  deterministic shell surface: the gathering banner, the URL output, and --help byte-parity. Help text is
  served by intercept_ts_help from the byte-exact fixture, not clap's auto-generated block.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to invoke `fspec report-bug-to-github` directly from a shell with the same surface offered by the
    TypeScript Commander.js CLI
    So that I can generate a pre-filled GitHub bug-report URL from a terminal without going through the LLM
    tool-call dispatcher

  Scenario: CLI prints the gathering banner and exits successfully
    Given an empty project root tempdir
    When I run `fspec report-bug-to-github --bug-description "crash on save"` in that directory
    Then the command exits with code 0
    And stdout contains "Gathering system context..."

  Scenario: CLI output includes the constructed GitHub issue URL
    Given an empty project root tempdir
    When I run `fspec report-bug-to-github --bug-description "crash on save"` in that directory
    Then the command exits with code 0
    And stdout contains "https://github.com/sengac/fspec/issues/new?title="

  Scenario: report-bug-to-github --help matches the TS formatCommandHelp reference
    Given the standalone fspec binary
    When I run `fspec report-bug-to-github --help`
    Then the command exits with code 0
    And stdout is byte-for-byte identical to tests/fixtures/help/report-bug-to-github.txt
