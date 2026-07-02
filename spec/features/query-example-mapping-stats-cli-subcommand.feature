@done
@querying
@cli
@RPC-260
Feature: query-example-mapping-stats clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `query-example-mapping-stats` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note [7] on RPC-253, reused for RPC-260):
  - Shell argv         → clap → codelet/fspec/src/query_example_mapping_stats.rs → fspec_core::commands::query_example_mapping_stats::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_example_mapping_stats::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes only `--format <text|json>` defaulting to `text` (parity with TS Commander.js registration at src/commands/query-example-mapping-stats.ts:163-165 — workUnitId/hasQuestions/questionsFor are NOT exposed on the CLI surface).
  Text format prints NOTHING to stdout (TS source-of-truth parity bug we replicate exactly — the TS CLI only prints when format==='json').
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::query_example_mapping_stats::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/query-example-mapping-stats.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a query-example-mapping-stats clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that aggregation logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes query-example-mapping-stats as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-example-mapping-stats --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'query-example-mapping-stats'
    Then stdout advertises the '--status' flag
    Then stdout does NOT contain the substring '--workUnitId'
    Then stdout does NOT contain the substring '--hasQuestions'
    Then stdout does NOT contain the substring '--questionsFor'

  Scenario: CLI with --format=json prints the canonical empty stats against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-example-mapping-stats --format json` from that directory
    Then the command exits 0
    Then stdout parses as JSON containing the fields workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit
    Then the parsed JSON workUnits is the empty array
    Then the parsed JSON has workUnitsWithRules=0, workUnitsWithExamples=0, workUnitsWithQuestions=0, workUnitsWithAssumptions=0

  Scenario: CLI without --format prints nothing to stdout (TS silent-text parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-example-mapping-stats` from that directory
    Then the command exits 0
    Then stdout is exactly empty
    Then stderr is exactly empty

  Scenario: CLI with --format=text also prints nothing to stdout (explicit text bug parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-example-mapping-stats --format text` from that directory
    Then the command exits 0
    Then stdout is exactly empty

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec query-example-mapping-stats --format json` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains AUTH-001 with 2 rules and 1 example
    When I dispatch query-example-mapping-stats through fspec_core::dispatch::dispatch_command with format='json'
    Then the DispatchResult.data parses as JSON with workUnitsWithRules=1 and workUnitsWithExamples=1
    Then the CLI bridge module codelet/fspec/src/query_example_mapping_stats.rs contains NO inline aggregation, filter, or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: query-example-mapping-stats --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-example-mapping-stats --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/query-example-mapping-stats.txt
    Then stdout starts with a blank line followed by 'QUERY-EXAMPLE-MAPPING-STATS'
