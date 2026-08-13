@done
@querying
@cli
@RPC-238
Feature: Port import-example-map command to Rust
  """
  Core: rust/fspec-core/src/commands/import_example_map.rs — pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>. Args: { workUnitId: String, file: String }. Reads spec/work-units.json via ensure_work_units_file (auto-create when missing, escalate malformed). Validates work unit exists ("Work unit '<id>' does not exist") AND is in 'specifying' state (else "Can only import example mapping during discovery/specification phase. <id> is in '<status>' state."). Reads the JSON file (ExampleMapData { rules?, examples?, questions?, assumptions? }: arrays of any). For each present array, APPENDS to the work unit's existing array (workUnit.x = [...(existing||[]), ...incoming]) and counts incoming length. Sets updatedAt = now. Writes work-units.json (2-space). Returns message "✓ Imported <total> items: <r> rules, <e> examples, <q> questions, <a> assumptions". This is the inverse of export-example-map (RPC-228).
  CLI bridge: rust/fspec/src/import_example_map.rs (CliArgs { work_unit_id, file }). clap variant Mode::ImportExampleMap with two required positionals. Success: println! the returned message; Error: eprintln! ✗ Failed to import example map: <msg>, exit 1. Help intercept + help config + fixture import-example-map.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/work-units.json via ensureWorkUnitsFile (auto-create when missing, escalate malformed JSON)
  #   2. Throws "Work unit '<id>' does not exist" when the work unit ID is not found
  #   3. Throws "Can only import example mapping during discovery/specification phase. <id> is in '<status>' state." when the work unit is not in specifying state
  #   4. Reads the import JSON file with fields rules, examples, questions, assumptions (each an optional array)
  #   5. Each present array is APPENDED to the work unit's existing array (existing items are preserved, new items concatenated); a count is recorded per category
  #   6. A missing or non-array category contributes zero imported items and leaves the existing array untouched
  #   7. The work unit updatedAt timestamp is refreshed and work-units.json is written with 2-space indent
  #   8. On success the CLI prints "✓ Imported <total> items: <r> rules, <e> examples, <q> questions, <a> assumptions" to stdout and exits 0
  #   9. On failure the CLI prints "✗ Failed to import example map: <message>" to stderr and exits 1
  #   10. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. Import a file with 2 rules, 3 examples, 1 question, 0 assumptions into specifying AUTH-001 appends them and returns ✓ Imported 6 items: 2 rules, 3 examples, 1 questions, 0 assumptions
  #   2. Import into AUTH-001 which already has 1 rule appends so the rules array now holds the original plus the imported items
  #   3. Import a file containing only examples leaves rules/questions/assumptions counts at 0 and untouched
  #   4. Import into NOPE-999 (not found) errors with "Work unit 'NOPE-999' does not exist"
  #   5. Import into a work unit in 'done' state errors with "Can only import example mapping during discovery/specification phase. AUTH-009 is in 'done' state."
  #   6. Round-trip: data exported by export-example-map can be re-imported by import-example-map
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run import-example-map in the Rust binary and via the LLM dispatcher
    So that the Rust port appends example-map data and emits byte-identical messages as the TypeScript command

  Scenario: Import full example mapping data into a specifying work unit
    Given a work units store where AUTH-001 is in specifying state with no example map data
    And an import file with two rules, three examples, one question, and zero assumptions
    When I import the example map from the file into AUTH-001
    Then the AUTH-001 rules, examples, and questions arrays contain the imported items
    And the returned message is "✓ Imported 6 items: 2 rules, 3 examples, 1 questions, 0 assumptions"

  Scenario: Import appends to existing example mapping arrays
    Given a work units store where AUTH-001 is in specifying state with one existing rule
    And an import file with two rules
    When I import the example map from the file into AUTH-001
    Then the AUTH-001 rules array holds the existing rule followed by the two imported rules

  Scenario: Import a file with only one category leaves the others untouched
    Given a work units store where AUTH-001 is in specifying state
    And an import file containing only examples
    When I import the example map from the file into AUTH-001
    Then only the examples count is non-zero and the rules, questions, and assumptions arrays are unchanged

  Scenario: Import into a work unit that does not exist fails
    Given a work units store that does not contain NOPE-999
    When I import the example map from a file into NOPE-999
    Then the run returns an error containing "Work unit 'NOPE-999' does not exist"

  Scenario: Import into a work unit not in specifying state fails
    Given a work units store where AUTH-009 is in done state
    When I import the example map from a file into AUTH-009
    Then the run returns an error containing "Can only import example mapping during discovery/specification phase. AUTH-009 is in 'done' state."

  Scenario: Dispatcher and core append identical data
    Given a work units store where AUTH-001 is in specifying state
    And an import file with one rule and one example
    When I import the example map for AUTH-001 via the core run function
    Then the resulting work unit state matches importing via the dispatcher path
