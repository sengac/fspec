@done
@querying
@parser
@RPC-207
Feature: Port compare-implementations command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/compare_implementations.rs reads spec/work-units.json (via the shared WorkUnitsData type; tags live in WorkUnit::extra["tags"]) and returns the JSON envelope {workUnits:[{tags}], comparison:{type:'side-by-side'}, namingConventionDifferences:[], coverage:[]}. Missing work-units.json surfaces FspecCoreError::Io (parity with show_test_patterns / the TS queryWorkUnits throw). With showCoverage=true every spec/features/*.feature.coverage file is read and coverage[0] = {testFiles, implementationFiles} carries the deduplicated testMapping / implMapping file paths; without it coverage stays empty. namingConventionDifferences is always empty (TS TODO). The CLI bridge owns all rendering (green summary / pretty JSON / '✗ Comparison failed:' error). The TS parseAllFeatures call is a no-op (its map is never consumed) and is intentionally omitted from the Rust port.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The --tag flag is REQUIRED; work units are read from spec/work-units.json via queryWorkUnits which throws if the file is missing
  #   2. Work units are filtered to those whose tags array contains the supplied tag; output is the envelope {workUnits:[{tags}], comparison:{type:'side-by-side'}, namingConventionDifferences:[], coverage:[]}
  #   3. With --show-coverage, all .feature.coverage files under spec/features are read; coverage[0].testFiles is the deduplicated set of testMapping file paths and coverage[0].implementationFiles is the deduplicated set of implMapping file paths; without --show-coverage the coverage array is empty
  #   4. namingConventionDifferences is always an empty array (TS TODO, not yet implemented); a missing spec/features directory yields empty coverage entries
  #   5. CLI default (no --json) prints green '✓ Compared N work units tagged with <tag>' and exits 0; --json prints the 2-space JSON envelope; on error (missing work-units.json) prints '✗ Comparison failed:' to stderr and exits 1
  #
  # EXAMPLES:
  #   1. Dispatcher with tags @cli over two tagged work units and --json returns workUnits length 2, comparison.type 'side-by-side', empty namingConventionDifferences and empty coverage
  #   2. With --show-coverage and one work unit tagged @cli plus a coverage file referencing one test file and one impl file, coverage[0].testFiles=['test/a.test.ts'] and coverage[0].implementationFiles=['src/a.ts']
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to run compare-implementations to summarise tag-filtered work units and optional coverage file paths via both the LLM dispatcher and the shell CLI
    So that I can audit implementation consistency with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: Dispatcher summarises work units carrying the tag
    Given a project root tempdir with spec/work-units.json containing two work units tagged @cli
    When I dispatch compare-implementations with tag=@cli
    Then the dispatcher returns workUnits with length 2
    And the comparison.type field equals 'side-by-side'
    And the namingConventionDifferences array is empty
    And the coverage array is empty

  Scenario: Dispatcher includes deduplicated coverage file paths
    Given a project root tempdir with spec/work-units.json containing one work unit tagged @cli and one .feature.coverage file referencing one test file and one impl file
    When I dispatch compare-implementations with tag=@cli and showCoverage=true
    Then the dispatcher returns coverage with one entry
    And coverage[0].testFiles equals ['test/a.test.ts']
    And coverage[0].implementationFiles equals ['src/a.ts']

  Scenario: Dispatcher returns empty workUnits when no tag matches
    Given a project root tempdir with spec/work-units.json containing one work unit tagged @other
    When I dispatch compare-implementations with tag=@cli
    Then the dispatcher returns workUnits with length 0
    And the coverage array is empty

  Scenario: Dispatcher errors when work-units.json is missing
    Given a project root tempdir with no spec/work-units.json
    When I dispatch compare-implementations with tag=@cli
    Then the dispatcher returns an error
