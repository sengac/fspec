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
  #   2. When neither --all nor --test-file is given the command errors 'Must specify either --all or --test-file'; when --impl-file is given without --test-file it errors '--test-file is required when specifying --impl-file'
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

  Scenario: Clap exposes unlink-coverage as a subcommand and prints flag help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec unlink-coverage --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'unlink-coverage'
    And stdout contains the substring '--scenario'

  Scenario: CLI --all empties the scenario mappings and prints the success message
    Given a temp workspace has a coverage sidecar where scenario "Login" has one test mapping
    When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Removed all coverage mappings for scenario "Login"'

  Scenario: CLI without --all or --test-file exits 1
    Given a temp workspace has a coverage sidecar with scenario "Login"
    When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login` from that workspace
    Then the command exits with a non-zero status
    And stderr contains the substring 'Error:'

  Scenario: CLI exits 1 when the coverage file is missing
    Given an empty directory with no coverage sidecar is the current working directory
    When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` from that directory
    Then the command exits with a non-zero status
    And stderr contains the substring 'Error:'

  Scenario: unlink-coverage --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec unlink-coverage --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/unlink-coverage.txt
    And stdout starts with a blank line followed by 'UNLINK-COVERAGE'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has unlink-coverage registered as a clap subcommand alongside other ported subcommands
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists unlink-coverage as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace has a coverage sidecar where scenario "Login" has one test mapping
    When I dispatch unlink-coverage through fspec_core::dispatch::dispatch_command for feature "user-login" with scenario='Login' and all=true against that workspace
    And I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` against an identical workspace
    Then both invocations report success
    And the CLI bridge module codelet/fspec/src/unlink_coverage.rs contains NO inline mutation or rendering logic — its only computation is JSON arg marshalling
