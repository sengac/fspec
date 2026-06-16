@done
@validation
@coverage-tracking
@cli
@RPC-231
Feature: Port generate-coverage command to Rust

  """
  Core impl: codelet/fspec-core/src/commands/generate_coverage.rs rewrites the stub; signature run(args_json, project_root). Reuses types/coverage.rs (CoverageFile/CoverageScenario/CoverageStats + calculate_stats) — REUSE only, no extension. Scans spec/features/*.feature; for each, resolves <feature>.feature.coverage and computes one of four statuses (created/recreated/updated/skipped) mirroring src/utils/coverage-file.ts createCoverageFile. Scenario names come from io::gherkin::parse_feature_lenient over top-level feature.scenarios. Created/recreated bodies are written via io::locked_file::write_json_atomic (2-space JSON, no trailing newline). Updated sidecars preserve existing test mappings + unknown stats fields (serde_json::Value path like delete_scenario::update_coverage), drop stale scenarios, add new empty ones, recompute totalScenarios/coveredScenarios/coveragePercent.
  Output (non-dry-run): a '✓ '-prefixed line joining nonzero parts 'Created N, Updated N, Skipped N, Recreated N (invalid JSON)' (or 'No coverage files needed'), ALWAYS followed verbatim by the long link-coverage <system-reminder> block (src/commands/generate-coverage.ts:155-189). Dry-run: 'Would create N coverage files (DRY RUN)' + file list + 'Would skip/recreate' lines, no writes, never reports updates. Missing spec/features dir → error 'Failed to read features directory'. The full stdout string is rendered in CORE and returned; the CLI bridge prints it verbatim.
  Two-front-doors: dispatcher and clap CLI both call generate_coverage::run. SHARED-FILE REQUEST (supervisor): move generate-coverage into run_ported and change the dispatch arm signature from run(args_json) to run(args_json, project_root); add to PORTED_COMMANDS in canonical.rs.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to generate or update empty .feature.coverage sidecars for existing feature files
    So that I can set up or resync coverage tracking without launching Node, sharing one Rust source of truth between the LLM dispatcher and the CLI

  Scenario: Creates a sidecar for a feature file that lacks one
    Given a temp project root has a feature file "user-login.feature" with two scenarios and no coverage sidecar
    When I dispatch generate-coverage against that project root
    Then the dispatcher returns success
    And a coverage sidecar "user-login.feature.coverage" is created with two scenario entries each having empty testMappings
    And the created sidecar stats report totalScenarios 2, coveredScenarios 0 and coveragePercent 0
    And the rendered output contains the substring "Created 1"
    And the rendered output contains the link-coverage system-reminder block

  Scenario: Skips a sidecar that is already in sync
    Given a temp project root has a feature file "user-login.feature" whose coverage sidecar already lists all its scenarios
    When I dispatch generate-coverage against that project root
    Then the dispatcher returns success
    And the rendered output contains the substring "Skipped 1"
    And the existing sidecar is left byte-for-byte unchanged

  Scenario: Updates a sidecar when scenarios were added and removed
    Given a temp project root has a feature file "user-login.feature" with one new scenario absent from its sidecar and one stale scenario only in the sidecar
    When I dispatch generate-coverage against that project root
    Then the dispatcher returns success
    And the updated sidecar adds the new scenario with empty testMappings and drops the stale scenario
    And the updated sidecar preserves the existing test mappings of unchanged scenarios
    And the rendered output contains the substring "Updated 1"

  Scenario: Recreates a sidecar that contains invalid JSON
    Given a temp project root has a feature file "user-login.feature" with a coverage sidecar whose contents are not valid JSON
    When I dispatch generate-coverage against that project root
    Then the dispatcher returns success
    And the sidecar is rewritten as valid JSON with one entry per scenario
    And the rendered output contains the substring "Recreated 1"

  Scenario: Dry-run reports would-create files without writing
    Given a temp project root has a feature file "user-login.feature" and no coverage sidecar
    When I dispatch generate-coverage with dryRun true against that project root
    Then the dispatcher returns success
    And no coverage sidecar is written to disk
    And the rendered output contains the substring "Would create 1 coverage files (DRY RUN)"
    And the rendered output lists "user-login.feature.coverage"

  Scenario: Missing features directory surfaces an error
    Given a temp project root has no spec/features directory
    When I dispatch generate-coverage against that project root
    Then the dispatcher returns an error whose message contains "Failed to read features directory"
