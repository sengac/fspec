@done
@querying
@cli
@RPC-235
Feature: Port generate-summary-report command to Rust
  """
  Core: codelet/fspec-core/src/commands/generate_summary_report.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { format: Option<String> (markdown|json, default markdown), output: Option<String> }. Reads spec/work-units.json directly via std::fs::read_to_string (NOT ensure — missing file is an error). Aggregates: totalWorkUnits, byStatus (insertion-order counts; status default "unknown"), totalStoryPoints (sum estimate||0), velocity { completedPoints, completedWorkUnits } over status=="done". Default output path spec/summary-report.<md|json>. json => JSON.stringify(report,null,2); markdown => generateMarkdownReport. Writes report to file. Returns the message "✓ Report generated: <outputFile>" (outputFile is the relative path). Any error wrapped as "Failed to generate summary report: <message>".
  CLI bridge: codelet/fspec/src/generate_summary_report.rs (CliArgs { format: Option<String>, output: Option<String> }). clap variant Mode::GenerateSummaryReport with --format and --output. Success: println! the returned message; Error: eprintln! ✗ Failed to generate report: <msg>, exit 1. Help intercept + help config + fixture generate-summary-report.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json directly via readFile; a missing or malformed file fails with "Failed to generate summary report: <message>"
  #   2. byStatus counts each work unit by its status field, defaulting missing status to "unknown", in work-unit insertion order
  #   3. totalStoryPoints sums each work unit estimate (treating missing estimate as 0)
  #   4. velocity.completedWorkUnits and velocity.completedPoints count and sum only work units whose status is "done"
  #   5. format defaults to markdown; "json" writes JSON.stringify(report, null, 2); markdown writes the generateMarkdownReport layout
  #   6. Default output path is spec/summary-report.md (markdown) or spec/summary-report.json (json); --output overrides it
  #   7. On success the CLI prints "✓ Report generated: <outputFile>" to stdout and exits 0; outputFile is the relative path passed/derived
  #   8. On failure the CLI prints "✗ Failed to generate report: <message>" to stderr and exits 1
  #   9. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. A store of 3 work units (2 done with estimates 3 and 5, 1 backlog estimate 2) produces totalWorkUnits 3, totalStoryPoints 10, velocity { completedPoints 8, completedWorkUnits 2 }
  #   2. A markdown report renders the heading, total work units, total story points, a per-status breakdown list, and velocity metrics
  #   3. A json report writes the report object pretty-printed with 2-space indent
  #   4. Generating with --output custom.md writes to custom.md and the returned outputFile is custom.md
  #   5. A work unit with no status is counted under "unknown" in byStatus
  #   6. A missing spec/work-units.json fails with "Failed to generate summary report:"
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run generate-summary-report in the Rust binary and via the LLM dispatcher
    So that the Rust port writes byte-identical summary reports and messages as the TypeScript command

  Scenario: Aggregate a store with completed and pending work units
    Given a work units store with two done units estimated 3 and 5 and one backlog unit estimated 2
    When I generate a json summary report
    Then the written report has totalWorkUnits 3 and totalStoryPoints 10
    And the velocity has completedPoints 8 and completedWorkUnits 2

  Scenario: Markdown report renders the expected layout
    Given a work units store with a mix of statuses and estimates
    When I generate a markdown summary report
    Then the written report begins with "# Project Summary Report"
    And the report includes a Breakdown by Status section and a Velocity Metrics section

  Scenario: JSON report is pretty-printed with two-space indent
    Given a work units store containing one work unit
    When I generate a json summary report
    Then the written report is JSON pretty-printed with two-space indentation

  Scenario: A custom output path is honoured
    Given a work units store containing one work unit
    When I generate a markdown summary report to custom.md
    Then the report is written to custom.md
    And the returned message is "✓ Report generated: custom.md"

  Scenario: A work unit without a status is counted as unknown
    Given a work units store where one work unit has no status field
    When I generate a json summary report
    Then the byStatus breakdown counts that work unit under "unknown"

  Scenario: A missing work units file fails
    Given a workspace with no spec/work-units.json file
    When I generate a summary report
    Then the run returns an error containing "Failed to generate summary report:"

  Scenario: Dispatcher and core produce identical report content
    Given a work units store containing one work unit
    When I generate a json summary report via the core run function
    Then the written report content is the same as generating via the dispatcher path
