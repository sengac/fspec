@done
@querying
@cli
@RPC-237
Feature: get-scenarios clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `get-scenarios` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note on RPC-253, reused for RPC-237):
  - Shell argv         → clap → codelet/fspec/src/get_scenarios.rs → fspec_core::commands::get_scenarios::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::get_scenarios::run
  Both call sites pass a JSON-encoded args shape ({tags, format}) and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS process.cwd() default).
  The clap subcommand exposes a repeatable --tag <tag> flag (collected into Vec<String>) and --format <format> defaulting to 'text' (parity with the TS Commander.js registration — note: registration does NOT register the --file flag the rich help advertises).
  format=json prints JSON.stringify(scenarios, null, 2) (the scenarios array only); format=text prints the message then scenarios grouped by feature.
  Exit-code contract: 0 on success, 1 on any FspecCoreError / success=false envelope with stderr prefixed 'Error:'.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/get-scenarios.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a get-scenarios clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that scenario-extraction logic is never duplicated and parity with the TS CLI is preserved

  Scenario: Clap exposes get-scenarios as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec get-scenarios --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'get-scenarios'

  Scenario: CLI with --format=json prints a JSON array of scenario objects
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    When I run `./codelet/target/release/fspec get-scenarios --tag @auth --format json` from that workspace
    Then the command exits 0
    Then stdout parses as a JSON array
    Then each array element has the keys feature, name, and line

  Scenario: CLI default text output prints the count message and groups scenarios by feature
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    When I run `./codelet/target/release/fspec get-scenarios --tag @auth` from that workspace
    Then the command exits 0
    Then stdout contains the substring 'Found 2 scenarios matching tags: @auth'
    Then stdout contains the substring 'spec/features/login.feature'

  Scenario: CLI against a workspace with no spec/features exits 1
    Given an empty directory with no spec/ subdirectory is the current working directory
    When I run `./codelet/target/release/fspec get-scenarios` from that directory
    Then the command exits 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'spec/features directory not found'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    When I dispatch get-scenarios through fspec_core::dispatch::dispatch_command with tags=['@auth'] and format='json'
    Then the DispatchResult succeeds and its data is a JSON array matching the CLI's --format json stdout
    Then the CLI bridge module codelet/fspec/src/get_scenarios.rs contains NO inline parsing, filtering, or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: get-scenarios --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec get-scenarios --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/get-scenarios.txt
    Then stdout starts with a blank line followed by 'GET-SCENARIOS'
