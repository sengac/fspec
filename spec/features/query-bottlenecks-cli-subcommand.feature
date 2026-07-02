@done
@cli
@querying
@RPC-256
@wip
Feature: query-bottlenecks clap subcommand on the standalone fspec Rust binary
  """
  CLI surface for the `query-bottlenecks` subcommand on the standalone fspec Rust binary.
  Two-front-doors pattern:
  - Shell argv         → clap → codelet/fspec/src/query_bottlenecks.rs → fspec_core::commands::query_bottlenecks::run
  - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::query_bottlenecks::run
  Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
  The CLI surface resolves project_root from CWD (parity with TS `process.cwd()` default).
  The clap subcommand exposes `--output <text|json>` defaulting to `text` (parity with TS Commander.js registration).
  Text format prints a multi-line summary; the empty case prints '✓ No bottlenecks found' verbatim.
  JSON format prints the pretty-printed JSON payload returned by fspec_core::commands::query_bottlenecks::run.
  Exit-code contract: 0 on success, 1 on any FspecCoreError with stderr prefixed `Error:`.
  --help is byte-for-byte identical to the captured TS fixture at codelet/fspec/tests/fixtures/help/query-bottlenecks.txt.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want a query-bottlenecks clap subcommand that delegates to the same fspec_core function the LLM dispatcher uses
    So that bottleneck-detection logic is never duplicated and byte-parity with the TS CLI is preserved

  Scenario: Clap exposes query-bottlenecks as a subcommand and prints flag-aware --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-bottlenecks --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'query-bottlenecks'

  Scenario: CLI without options prints '✓ No bottlenecks found' against an empty workspace
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-bottlenecks` from that directory
    Then the command exits 0
    And stdout contains the exact line '✓ No bottlenecks found'

  Scenario: CLI with --output=json prints empty bottlenecks array
    Given an empty directory with no spec/ subdirectory is set as the current working directory
    When I run `./codelet/target/release/fspec query-bottlenecks --output json` from that directory
    Then the command exits 0
    And stdout parses as JSON whose root object has bottlenecks=[]

  Scenario: CLI text output lists each qualifying bottleneck with its score
    Given a workspace whose spec/work-units.json contains A with blocks=['B','C'] and B with blocks=['D'] and C and D with no dependency fields
    When I run `./codelet/target/release/fspec query-bottlenecks` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Bottleneck Work Units (blocking 2+ work units):'
    And stdout contains the substring 'A'
    And stdout contains the substring 'Bottleneck Score: 3'
    And stdout contains the substring 'Total bottlenecks: 1'

  Scenario: CLI exits 1 and writes to stderr when work-units.json is malformed
    Given spec/work-units.json exists in the working directory but contains invalid JSON
    When I run `./codelet/target/release/fspec query-bottlenecks --output json` from that directory
    Then the command exits with code 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring 'Failed to parse work-units.json'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a workspace whose spec/work-units.json contains A with blocks=['B','C'] and B with blocks=['D']
    When I dispatch query-bottlenecks through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    And I run `./codelet/target/release/fspec query-bottlenecks --output json` against the same workspace
    Then both invocations produce JSON with bottlenecks[0].id='A' and bottlenecks[0].score=3
    And the CLI bridge module codelet/fspec/src/query_bottlenecks.rs contains NO inline DFS, filtering, or rendering logic — its only computation is JSON arg marshalling and stdout printing

  Scenario: query-bottlenecks --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec query-bottlenecks --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/query-bottlenecks.txt
    And stdout starts with a blank line followed by 'QUERY-BOTTLENECKS'
