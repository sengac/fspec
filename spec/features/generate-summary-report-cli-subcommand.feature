@done
@querying
@cli
@RPC-235
Feature: generate-summary-report CLI subcommand
  """
  CLI bridge: codelet/fspec/src/generate_summary_report.rs (CliArgs { format: Option<String>, output: Option<String> }). clap variant Mode::GenerateSummaryReport with --format <format> and --output <file>. Resolves project_root from CWD (parity with TS process.cwd()). Success: println! the returned "✓ Report generated: <outputFile>" message and exit 0; Error: eprintln! "✗ Failed to generate report: <msg>" and exit 1. Help intercept renders help/configs/generate_summary_report.rs; fixture codelet/fspec/tests/fixtures/help/generate-summary-report.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. `fspec generate-summary-report` with no flags writes spec/summary-report.md and prints "✓ Report generated: spec/summary-report.md"
  #   2. --format json writes spec/summary-report.json by default
  #   3. --output <file> overrides the default output path
  #   4. On failure (e.g. missing work-units.json) the CLI prints "✗ Failed to generate report: <message>" to stderr and exits 1
  #   5. `--help` prints the captured help fixture byte-for-byte
  #   6. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. Running with --output report.md in a populated workspace exits 0 and writes report.md
  #   2. Running --format json writes spec/summary-report.json
  #   3. Running in a workspace with no spec/work-units.json exits 1 with the failure message
  #   4. Running --help prints the help fixture
  #   5. CLI and dispatcher write identical report content for the same store
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run generate-summary-report in the Rust binary and via the LLM dispatcher
    So that the Rust port writes byte-identical summary reports and messages as the TypeScript command

  Scenario: CLI generate-summary-report writes the report and prints the success message
    Given a workspace whose spec/work-units.json contains a few work units
    When I run `fspec generate-summary-report --output report.md`
    Then the command exits with code 0
    And stdout contains "✓ Report generated: report.md"
    And report.md contains the rendered summary report

  Scenario: CLI generate-summary-report --format json writes the json report
    Given a workspace whose spec/work-units.json contains a few work units
    When I run `fspec generate-summary-report --format json`
    Then the command exits with code 0
    And spec/summary-report.json contains the pretty-printed report JSON

  Scenario: CLI generate-summary-report fails when the work units file is missing
    Given an empty workspace with no spec/work-units.json
    When I run `fspec generate-summary-report`
    Then the command exits with code 1
    And stderr contains "✗ Failed to generate report:"

  Scenario: CLI generate-summary-report --help prints the help fixture
    Given an empty workspace
    When I run `fspec generate-summary-report --help`
    Then stdout matches the captured generate-summary-report help fixture

  Scenario: CLI delegates to the same fspec-core function as the dispatcher
    Given a workspace whose spec/work-units.json contains a few work units
    When I generate a report via the CLI and via the dispatcher into separate files
    Then both files have identical content
