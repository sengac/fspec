@done
@querying
@cli
@RPC-238
Feature: import-example-map CLI subcommand
  """
  CLI bridge: codelet/fspec/src/import_example_map.rs (CliArgs { work_unit_id, file }). clap variant Mode::ImportExampleMap with two required positionals <workUnitId> <file>. Resolves project_root from CWD (parity with TS process.cwd()). Success: println! the returned "✓ Imported <total> items: ..." message and exit 0; Error: eprintln! "✗ Failed to import example map: <msg>" and exit 1. Help intercept renders help/configs/import_example_map.rs; fixture codelet/fspec/tests/fixtures/help/import-example-map.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. `fspec import-example-map <workUnitId> <file>` appends example-map data and prints the success message on exit 0
  #   2. On failure (unknown work unit, wrong state, missing file) the CLI prints "✗ Failed to import example map: <message>" to stderr and exits 1
  #   3. Missing the required <file> argument exits non-zero with a missing-argument message
  #   4. `--help` prints the captured help fixture byte-for-byte
  #   5. Both invocation paths (CLI clap subcommand and LLM dispatcher) converge on the same fspec-core run function
  #
  # EXAMPLES:
  #   1. Running with AUTH-001 and a valid import file exits 0 and prints ✓ Imported ... message
  #   2. Running with NOPE-999 exits 1 with the not-found failure message
  #   3. Running without the file argument exits non-zero with a missing-argument message
  #   4. Running --help prints the help fixture
  #   5. CLI and dispatcher append identical data for the same store and file
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting the CLI to Rust
    I want to run import-example-map in the Rust binary and via the LLM dispatcher
    So that the Rust port appends example-map data and emits byte-identical messages as the TypeScript command

  Scenario: CLI import-example-map appends data and prints the success message
    Given a workspace whose spec/work-units.json has AUTH-001 in specifying state and an import file
    When I run `fspec import-example-map AUTH-001 emap.json`
    Then the command exits with code 0
    And stdout contains "✓ Imported"
    And spec/work-units.json now contains the imported items under AUTH-001

  Scenario: CLI import-example-map fails for an unknown work unit
    Given a workspace whose spec/work-units.json does not contain NOPE-999 and an import file
    When I run `fspec import-example-map NOPE-999 emap.json`
    Then the command exits with code 1
    And stderr contains "✗ Failed to import example map: Work unit 'NOPE-999' does not exist"

  Scenario: CLI import-example-map requires the file argument
    Given an empty workspace
    When I run `fspec import-example-map AUTH-001`
    Then the command exits with a non-zero code
    And stderr reports a missing required argument

  Scenario: CLI import-example-map --help prints the help fixture
    Given an empty workspace
    When I run `fspec import-example-map --help`
    Then stdout matches the captured import-example-map help fixture

  Scenario: CLI delegates to the same fspec-core function as the dispatcher
    Given a workspace whose spec/work-units.json has AUTH-001 in specifying state and an import file
    When I import AUTH-001 via the CLI and via the dispatcher into separate stores
    Then both stores have identical AUTH-001 example map data
