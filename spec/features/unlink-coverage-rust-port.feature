@validation
@coverage-tracking
@cli
@wip
@RPC-311
Feature: Port unlink-coverage command to Rust
  """
  Core impl: codelet/fspec-core/src/commands/unlink_coverage.rs rewrites stub; signature run(args_json, project_root). Reuses types/coverage.rs (CoverageFile/CoverageScenario/TestMapping/ImplMapping/CoverageStats). Reads sidecar via std::fs::read_to_string + serde_json; mutates in memory; LOCAL update_stats (NOT shared calculate_stats — totalLinesCovered must sum test ranges + impl line counts). Writes back via io::locked_file::write_json_atomic (no trailing newline). extra-flatten preserves unknown fields.
  Two-front-doors: dispatcher and clap CLI both call unlink_coverage::run. CLI bridge codelet/fspec/src/unlink_coverage.rs marshals positional feature-name + --scenario/--test-file/--impl-file/--all into JSON only. Help config codelet/fspec-core/src/help/configs/unlink_coverage.rs (unlink-coverage-help.ts rich help exists; help-config common_errors use CommonError type) + intercept arm + Mode::UnlinkCoverage variant wired by supervisor. SHARED-FILE REQUEST: dispatch arm must pass project_root (signature changes from run(args_json) to run(args_json, project_root)).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The feature-name positional and --scenario are required; --test-file, --impl-file and --all are optional
  #   2. When neither --all nor --test-file is given the command errors 'Must specify either --all or --test-file'; this check fires first, so passing --impl-file without --test-file (and without --all) also surfaces that same message (TS parity: the !all && !testFile guard precedes the impl-file guard, making the latter unreachable via the CLI)
  #   3. The coverage sidecar resolves to spec/features/<feature-name>.feature.coverage (the .feature suffix is tolerated); a missing file errors 'Coverage file not found'; a scenario not present in the file errors 'Scenario not found' and lists available scenarios
  #   4. The all flag empties the scenario testMappings; test-file alone removes that test mapping and all its impl mappings; test-file plus impl-file removes only the matching impl mapping; a missing test-file or impl-file errors with a not-found message
  #   5. After mutation the stats block is recalculated: coveredScenarios counts scenarios with testMappings, coveragePercent is Math.round(covered/total*100), testFiles and implFiles are deduplicated in insertion order, and totalLinesCovered sums test line ranges plus impl line counts
  #   6. The updated coverage file is written back atomically as 2-space JSON without dropping unknown fields; on success the CLI prints the result message and exits 0, on error it prints 'Error:' to stderr and exits 1
  #
  # EXAMPLES:
  #   1. Running unlink-coverage user-login --scenario "Login" --all empties the scenario's testMappings and drops its coveragePercent
  #   2. Running unlink-coverage user-login --scenario "Login" --test-file src/auth.test.ts removes the whole test mapping including its impl mappings
  #   3. Running unlink-coverage user-login --scenario "Login" --test-file src/auth.test.ts --impl-file src/old.ts removes only the impl mapping, keeping the test mapping
  #
  # ========================================
  Background: User Story
    As a developer managing coverage tracking via the standalone fspec Rust binary
    I want to remove test or implementation mappings from a scenario's coverage sidecar and recalculate stats, sharing one Rust source of truth between the LLM dispatcher and the CLI
    So that I can correct or reset coverage as code evolves without manual JSON editing or launching Node

  Scenario: --all empties the scenario testMappings and recalculates stats
    Given a temp project root has a coverage sidecar where scenario "Login" has one test mapping with impl mappings
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    Then the dispatcher returns success=true
    And the scenario "Login" testMappings array is empty in the written sidecar
    And the stats coveragePercent reflects the removed coverage

  Scenario: --test-file removes the whole test mapping including impl mappings
    Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts" with impl mappings
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and testFile="src/auth.test.ts"
    Then the dispatcher returns success=true
    And the scenario "Login" has no test mapping for "src/auth.test.ts" in the written sidecar

  Scenario: --test-file with --impl-file removes only the impl mapping
    Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts" with an impl mapping for "src/old.ts"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login", testFile="src/auth.test.ts" and implFile="src/old.ts"
    Then the dispatcher returns success=true
    And the test mapping for "src/auth.test.ts" still exists in the written sidecar
    And that test mapping no longer references "src/old.ts"

  Scenario: Neither --all nor --test-file surfaces a validation error
    Given a temp project root has a coverage sidecar with scenario "Login"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and no test-file or all flag
    Then the dispatcher returns success=false
    And the error field contains the substring 'Must specify either --all or --test-file'

  Scenario: --impl-file without --test-file or --all surfaces the required-flag error
    Given a temp project root has a coverage sidecar with scenario "Login"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and implFile="src/old.ts" and no test-file
    Then the dispatcher returns success=false
    And the error field contains the substring 'Must specify either --all or --test-file'

  Scenario: Missing coverage file surfaces a not-found error
    Given a temp project root has no coverage sidecar for feature "user-login"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    Then the dispatcher returns success=false
    And the error field contains the substring 'Coverage file not found'

  Scenario: Unknown scenario surfaces a not-found error
    Given a temp project root has a coverage sidecar with scenario "Login"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Logout" and all=true
    Then the dispatcher returns success=false
    And the error field contains the substring 'Scenario not found'

  Scenario: Unknown test file surfaces a not-found error
    Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and testFile="src/missing.test.ts"
    Then the dispatcher returns success=false
    And the error field contains the substring 'Test file not found in scenario mappings'

  Scenario: Atomic write back preserves unknown fields in the sidecar
    Given a temp project root has a coverage sidecar carrying an unknown top-level field alongside scenario "Login"
    When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    Then the dispatcher returns success=true
    And the written sidecar still contains the unknown top-level field

  Scenario: Shared infrastructure module is registered for unlink-coverage
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/commands/unlink_coverage.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes unlink-coverage to the new run function
