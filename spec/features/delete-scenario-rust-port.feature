@done
@feature-management
@parser
@RPC-219
Feature: Port delete-scenario command to Rust
  """
  Core impl at rust/fspec-core/src/commands/delete_scenario.rs uses crate::io::gherkin::parse_feature_lenient for parse + re-validate, and gherkin-0.16 Scenario.position.line / Step.position.line (1-based) to compute the removal span; line-based split('\n')/join('\n') edit.
  Coverage sidecar update reuses crate::types::coverage::{CoverageFile, CoverageScenario}; only totalScenarios/coveredScenarios/coveragePercent are recomputed (Math.round half-up), other stats fields preserved via serde flatten extra — matching the TS spread of ...coverage.stats.
  Recoverable failures returned as inner JSON envelope {success:false,error} (NOT FspecCoreError) like list_scenario_tags; CLI bridge prints '✓ <message>' on success / 'Error: <error>' to stderr + exit 1. Two-front-doors: bridge marshals positional <feature> <scenario> into {feature, scenario} JSON only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Feature path resolves: ends-with .feature OR starts-with spec/features/ → join(cwd, feature); else join(cwd, 'spec/features', feature + '.feature')
  #   2. Missing target file MUST return success=false with error 'Feature file not found: <absPath>'
  #   3. Unparseable Gherkin MUST return error 'Invalid Gherkin syntax: <msg>'; a parsed-but-featureless file MUST return 'Feature file does not contain a valid Feature'
  #   4. A scenario name with no match MUST return error "Scenario '<name>' not found in feature file"
  #   5. Removal span = scenario start line through last step line, extended forward over trailing blank lines but stopping at the next Scenario/Scenario Outline/Background/Feature/Examples header
  #   6. After removal, consecutive blank lines MUST be collapsed to at most 2; content is split/joined on '\n'
  #   7. If the post-deletion content fails to re-parse, MUST return error 'Deletion would result in invalid Gherkin: <msg>' and NOT write the file
  #   8. On success with no coverage sidecar, message = "Successfully deleted scenario '<name>' from <fileName>"
  #   9. When a <file>.coverage sidecar exists, the deleted scenario MUST be removed from coverage.scenarios and stats (totalScenarios, coveredScenarios, coveragePercent via Math.round) recomputed while preserving other fields; message gains '\n  Updated coverage file'
  #   10. A malformed or unreadable coverage sidecar MUST be ignored — deletion still succeeds with the no-coverage message
  #
  # EXAMPLES:
  #   1. Dispatcher deletes 'Old scenario' from a 2-scenario feature: success=true, file no longer contains the scenario, other scenario intact
  #   2. Deleting a scenario named 'Missing' returns success=false, error "Scenario 'Missing' not found in feature file"
  #   3. Deleting against spec/features/missing.feature returns success=false, error starts 'Feature file not found:'
  #   4. With a login.feature.coverage holding two scenarios, deleting one removes it from coverage.scenarios, recomputes coveragePercent, and message ends '  Updated coverage file'
  #   5. CLI: `fspec delete-scenario spec/features/login.feature "Old"` exits 0 and prints '✓ Successfully deleted scenario'
  #   6. CLI on a missing scenario exits 1 with stderr 'Error:' prefix
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to delete a named scenario from a feature file (and update its coverage sidecar) via both the LLM dispatcher and the shell CLI
    So that I can prune obsolete scenarios with byte-for-byte parity to the TypeScript implementation without relying on Node.js

  Scenario: Dispatcher deletes a scenario from a two-scenario feature
    Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    Then the dispatcher returns success=true
    And the file on disk no longer contains 'Scenario: Old scenario'
    And the file on disk still contains 'Scenario: Keep scenario'

  Scenario: Dispatcher reports a missing scenario name
    Given a project root tempdir with spec/features/login.feature containing a scenario 'Keep scenario'
    When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Missing'
    Then the dispatcher returns success=false
    And the dispatcher error equals "Scenario 'Missing' not found in feature file"

  Scenario: Dispatcher reports a missing feature file
    Given a project root tempdir with no spec/features/missing.feature
    When I dispatch delete-scenario with feature='spec/features/missing.feature' and scenario='Old scenario'
    Then the dispatcher returns success=false
    And the dispatcher error starts with 'Feature file not found:'

  Scenario: Dispatcher updates the coverage sidecar when present
    Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    And a spec/features/login.feature.coverage sidecar listing both scenarios
    When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    Then the dispatcher returns success=true
    And the coverage sidecar no longer lists 'Old scenario'
    And the dispatcher message ends with '  Updated coverage file'

  Scenario: Dispatcher ignores a malformed coverage sidecar
    Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    And a spec/features/login.feature.coverage sidecar containing invalid JSON
    When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    Then the dispatcher returns success=true
    And the dispatcher message equals "Successfully deleted scenario 'Old scenario' from login.feature"
