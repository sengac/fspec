@done
@querying
@cli
@RPC-307
Feature: Port show-test-patterns command to Rust
  """
  Reuses io::ensure::ensure_work_units_file path (TS queryWorkUnits throws on ENOENT — Rust mirrors by bubbling). Tag filter mirrors queryWorkUnits filter logic at src/commands/query-work-units.ts:118-123. No tag normalization (raw string includes).
  Adds NEW io helper: read_all_coverage_files(project_root) → Vec<CoverageFile> in io/coverage_glob.rs. Globs spec/features/*.feature.coverage, parses each JSON, returns minimal struct {feature_name, file_path, scenarios:[{name, testMappings:[{file, lines}]}]}. Skip files with parse errors. Submit shared-file request to wire this into io/mod.rs.
  Feature parsing for workUnitId tag extraction uses gherkin crate + the @PREFIX-NNN regex pattern from show_feature.rs. Parsed feature result is built but NOT used downstream in TS (placeholder for future); Rust mirrors this pass-through with a documented TODO.
  Dispatcher returns String envelope (JSON object) regardless of format flag; CLI bridge inspects --json to either pretty-print the JSON or print the green ✓ summary text line. Two-front-doors: core returns 2-indented JSON always; bridge swaps text/json rendering.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch show-test-patterns to filter work units by tag and report shared testing patterns (optionally including test file paths from coverage data)
    So that I can audit testing consistency across tagged work units without launching Node, sharing one Rust source of truth between the LLM dispatcher and the CLI

  Scenario: Missing tag argument surfaces a structured InvalidArgs error
    Given a temp project root contains a valid spec/work-units.json
    When I dispatch show-test-patterns with no tag argument
    Then the dispatcher returns success=false
    And the error field contains the substring 'tag'

  Scenario: Tag matches zero work units returns empty workUnits and patterns
    Given a temp project root contains spec/work-units.json with two work units neither tagged @missing
    When I dispatch show-test-patterns with tag='@missing'
    Then the dispatcher returns success=true
    And the data.workUnits array is empty
    And the data.patterns array is empty
    And the data.testFiles array is empty
    And the data.format equals 'table'

  Scenario: Tag matches work units returns their tags arrays
    Given a temp project root contains spec/work-units.json with two work units tagged @cli and one untagged
    When I dispatch show-test-patterns with tag='@cli'
    Then the dispatcher returns success=true
    And the data.workUnits array has 2 elements
    And every workUnits[i].tags array contains '@cli'

  Scenario: includeCoverage true reads coverage files and dedupes test files
    Given a temp project root contains spec/work-units.json with one work unit tagged @cli and two .feature.coverage files referencing three unique testMappings file paths
    When I dispatch show-test-patterns with tag='@cli' and includeCoverage=true
    Then the dispatcher returns success=true
    And the data.testFiles array has 3 unique elements

  Scenario: includeCoverage false leaves testFiles empty even when coverage files exist
    Given a temp project root contains spec/work-units.json with one work unit tagged @cli and one .feature.coverage file referencing testMappings paths
    When I dispatch show-test-patterns with tag='@cli' and no includeCoverage flag
    Then the dispatcher returns success=true
    And the data.testFiles array is empty

  Scenario: json format flag sets data.format to 'json'
    Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    When I dispatch show-test-patterns with tag='@cli' and json=true
    Then the dispatcher returns success=true
    And the data.format equals 'json'

  Scenario: Default format flag yields 'table' format
    Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    When I dispatch show-test-patterns with tag='@cli' and no json flag
    Then the dispatcher returns success=true
    And the data.format equals 'table'

  Scenario: Shared infrastructure module is registered for show-test-patterns
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/commands/show_test_patterns.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes show-test-patterns to the new run function
