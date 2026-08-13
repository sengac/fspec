@done
@querying
@cli
@RPC-228
Feature: Port export-example-map command to Rust
  """
  Core: rust/fspec-core/src/commands/export_example_map.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { workUnitId: String, file: String }. Reads rules/examples/questions/assumptions from WorkUnit.extra as serde_json::Value; defaults to empty arrays. Serializes via #[derive(Serialize)] struct (fixed field order) with to_string_pretty (2-space).
  CLI bridge: rust/fspec/src/export_example_map.rs (CliArgs { work_unit_id, file }). clap variant Mode::ExportExampleMap with two required positionals. Success: println! the returned message; Error: eprintln! ✗ Failed to export example map: <msg>, exit 1. Help intercept + help config rust/fspec-core/src/help/configs/export_example_map.rs + fixture export-example-map.txt.
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

  Scenario: Export a work unit with full example mapping data
    Given a work units store where AUTH-001 has one rule, one example, one question, and one assumption
    When I export the example map for AUTH-001 to emap.json
    Then the written JSON has fields workUnitId, title, rules, examples, questions, and assumptions in that order
    And the rules, examples, questions, and assumptions arrays each contain their single item verbatim
    And the returned message is "✓ Exported to emap.json"

  Scenario: Export a work unit with no example mapping data
    Given a work units store where AUTH-002 has no rules, examples, questions, or assumptions
    When I export the example map for AUTH-002 to emap2.json
    Then the written JSON has rules, examples, questions, and assumptions all as empty arrays

  Scenario: Export to a nested output path creates parent directories
    Given a work units store containing AUTH-001
    When I export the example map for AUTH-001 to out/maps/emap.json
    Then the parent directory out/maps is created and the file is written

  Scenario: Export a work unit that does not exist fails
    Given a work units store that does not contain NOPE-999
    When I export the example map for NOPE-999 to out.json
    Then the run returns an error containing "Work unit 'NOPE-999' does not exist"

  Scenario: Export with a malformed work units file escalates a parse error
    Given a spec/work-units.json file that is not valid JSON
    When I export the example map for AUTH-001 to out.json
    Then the run returns an error containing "Failed to parse work-units.json"

  Scenario: Dispatcher and core produce identical file content
    Given a work units store containing AUTH-001
    When I export the example map for AUTH-001 via the core run function
    Then the written file content is the same as exporting via the dispatcher path
