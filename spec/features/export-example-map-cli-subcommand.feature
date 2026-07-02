@done
@querying
@cli
@RPC-228
Feature: Port export-example-map command to Rust
  """
  Core: codelet/fspec-core/src/commands/export_example_map.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { workUnitId: String, file: String }. Reads rules/examples/questions/assumptions from WorkUnit.extra as serde_json::Value; defaults to empty arrays. Serializes via #[derive(Serialize)] struct (fixed field order) with to_string_pretty (2-space).
  CLI bridge: codelet/fspec/src/export_example_map.rs (CliArgs { work_unit_id, file }). clap variant Mode::ExportExampleMap with two required positionals. Success: println! the returned message; Error: eprintln! ✗ Failed to export example map: <msg>, exit 1. Help intercept + help config codelet/fspec-core/src/help/configs/export_example_map.rs + fixture export-example-map.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json via ensureWorkUnitsFile (auto-create when missing, escalate malformed JSON)
  #   2. Throws "Work unit '<id>' does not exist" when the work unit ID is not found
  #   3. Export JSON has fixed field order: workUnitId, title, rules, examples, questions, assumptions
  #   4. Missing example-map arrays default to empty arrays; present items round-trip verbatim from disk
  #   5. Output file is written with 2-space indent (JSON.stringify(data, null, 2)) after creating parent directories recursively
  #   6. On success the CLI prints "✓ Exported to <file>" to stdout and exits 0
  #   7. On failure the CLI prints "✗ Failed to export example map: <message>" to stderr and exits 1
  #   8. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. Export AUTH-001 (with 1 rule, 1 example, 1 question, 1 assumption) writes JSON with all four populated arrays and prints ✓ Exported to emap.json
  #   2. Export a work unit with no example-map data writes JSON with rules/examples/questions/assumptions all as empty arrays
  #   3. Export to a nested path like out/maps/emap.json creates the parent directories before writing
  #   4. Export NOPE-999 (not found) exits 1 with stderr ✗ Failed to export example map: Work unit 'NOPE-999' does not exist
  #   5. Export with malformed spec/work-units.json escalates a parse error (Failed to parse work-units.json)
  #   6. Dispatcher and CLI produce identical written file content for the same work unit
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run export-example-map in the Rust binary and via the LLM dispatcher
    So that the Rust port emits byte-identical example-map JSON and messages as the TypeScript command

  Scenario: CLI export-example-map writes the JSON file and prints the success message
    Given a workspace whose spec/work-units.json contains AUTH-001 with example mapping data
    When I run `fspec export-example-map AUTH-001 emap.json`
    Then the command exits with code 0
    And stdout contains "✓ Exported to emap.json"
    And emap.json contains the exported example mapping JSON

  Scenario: CLI export-example-map fails for an unknown work unit
    Given a workspace whose spec/work-units.json does not contain NOPE-999
    When I run `fspec export-example-map NOPE-999 out.json`
    Then the command exits with code 1
    And stderr contains "✗ Failed to export example map: Work unit 'NOPE-999' does not exist"

  Scenario: CLI export-example-map requires the file argument
    Given an empty workspace
    When I run `fspec export-example-map AUTH-001`
    Then the command exits with a non-zero code
    And stderr reports a missing required argument

  Scenario: CLI export-example-map --help prints the help fixture
    Given an empty workspace
    When I run `fspec export-example-map --help`
    Then stdout matches the captured export-example-map help fixture

  Scenario: CLI delegates to the same fspec-core function as the dispatcher
    Given a workspace whose spec/work-units.json contains AUTH-001 with example mapping data
    When I export AUTH-001 via the CLI and via the dispatcher into separate files
    Then both files have identical content
