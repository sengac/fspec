@done
@cli
@querying
@RPC-257
Feature: query-dependency-stats clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `query-dependency-stats` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern (architecture note [7] on RPC-253, reused for RPC-257):
  - Shell argv         → clap → codelet/fspec/src/query_dependency_stats.rs → fspec_core::commands::query_dependency_stats::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_dependency_stats::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes `--format <text|json>` defaulting to `text` (parity with TS Commander.js registration).
  Text format prints NOTHING to stdout (TS source-of-truth parity bug we replicate exactly — the TS CLI only prints when format==='json').
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::query_dependency_stats::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/query-dependency-stats.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a query-dependency-stats clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that aggregation logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes query-dependency-stats as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-dependency-stats --help` from a shell
    Then the command exits 0
    Then stdout contains the substring 'query-dependency-stats'

  Scenario: CLI with --format=json prints all ten canonical fields against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-dependency-stats --format json` from that directory
    Then the command exits 0
    Then stdout parses as JSON containing the fields totalBlocks, totalBlockedBy, totalDependsOn, totalRelatesTo, workUnitsWithDependencies, workUnitsWithBlockers, workUnitsBlockingOthers, workUnitsWithSoftDependencies, averageDependenciesPerUnit, maxDependencyChainDepth
    Then every field except maxDependencyChainDepth is the JSON number 0
    Then maxDependencyChainDepth is the JSON number 0

  Scenario: CLI without --format prints nothing to stdout (TS silent-text parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-dependency-stats` from that directory
    Then the command exits 0
    Then stdout is exactly empty
    Then stderr is exactly empty

  Scenario: CLI with --format=text also prints nothing to stdout (explicit text bug parity)
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-dependency-stats --format text` from that directory
    Then the command exits 0
    Then stdout is exactly empty

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec query-dependency-stats --format json` from that directory
    Then the command exits with code 1
    Then stderr contains the substring 'Error:'
    Then stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root whose spec/work-units.json contains A with blocks=['B'] and B with no dependency fields
    When I dispatch query-dependency-stats through fspec_core::dispatch::dispatch_command with format='json'
    Then the DispatchResult.data parses as JSON with totalBlocks=1 and maxDependencyChainDepth=1
    Then the CLI bridge module codelet/fspec/src/query_dependency_stats.rs contains NO inline aggregation, DFS, or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: query-dependency-stats --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-dependency-stats --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/query-dependency-stats.txt
    Then stdout starts with a blank line followed by 'QUERY-DEPENDENCY-STATS'
