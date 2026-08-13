@done
@cli
@querying
@RPC-259
@wip
Feature: query-estimation-guide clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `query-estimation-guide` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → rust/fspec/src/query_estimation_guide.rs → fspec_core::commands::query_estimation_guide::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_estimation_guide::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes the REQUIRED positional <workUnitId> argument (TS parity — argument is accepted but unused by the core function) and `--format <text|json>` defaulting to `text`.
  Text format prints NOTHING to stdout (TS source-of-truth parity bug we replicate exactly — the TS CLI only prints when format==='json').
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::query_estimation_guide::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at rust/fspec/tests/fixtures/help/query-estimation-guide.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a query-estimation-guide clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that estimation-guidance logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes query-estimation-guide as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-estimation-guide --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'query-estimation-guide'

  Scenario: CLI without --format prints nothing to stdout (TS silent-text parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-estimation-guide ANY-001` from that directory
    Then the command exits 0
    And stdout is exactly empty
    And stderr is exactly empty

  Scenario: CLI with --format=text also prints nothing to stdout (explicit text bug parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-estimation-guide ANY-001 --format text` from that directory
    Then the command exits 0
    And stdout is exactly empty

  Scenario: CLI with --format=json prints empty patterns array against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-estimation-guide ANY-001 --format json` from that directory
    Then the command exits 0
    And stdout parses as JSON whose root object has patterns=[]

  Scenario: CLI requires the positional workUnitId argument
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./rust/target/release/fspec query-estimation-guide` from that directory with no positional argument
    Then the command exits with a non-zero code

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./rust/target/release/fspec query-estimation-guide ANY-001 --format json` from that directory
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace whose spec/work-units.json contains a single done unit with estimate=3, iterations=1
    When I dispatch query-estimation-guide through fspec_core::dispatch::dispatch_command with workUnitId='ANY-001' and format='json' against that workspace
    And I run `./rust/target/release/fspec query-estimation-guide ANY-001 --format json` against the same workspace
    Then both invocations produce JSON with patterns[0].points=3 and patterns[0].confidence='low'
    And the CLI bridge module rust/fspec/src/query_estimation_guide.rs contains NO inline grouping, bucketing, or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: query-estimation-guide --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec query-estimation-guide --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-estimation-guide.txt
    And stdout starts with a blank line followed by 'QUERY-ESTIMATION-GUIDE'
