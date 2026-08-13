@done
@validation
@coverage-tracking
@cli
@RPC-240
Feature: Port link-coverage command to Rust
  """
  Core impl: rust/fspec-core/src/commands/link_coverage.rs rewrites the stub; signature run(args_json, project_root). Reuses types/coverage.rs (CoverageFile/CoverageScenario/TestMapping/ImplMapping/ImplLines) — REUSE only, no extension. LOCAL update_stats (mirrors src/commands/link-coverage/stats-updater.ts; totalLinesCovered = test line ranges + impl array lengths; NOT shared calculate_stats) duplicated inside this module (must not touch unlink_coverage.rs). parseImplLines ports comma/range string → Vec<i64> stored as ImplLines::Array. Step validation ports src/utils/step-validation.ts + similarity-algorithms.ts (jaroWinkler, tokenSet, trigram, jaccard, gherkinStructural, weighted hybrid, adaptive thresholds 0.85/<10, 0.80/<20, 0.75/<40, 0.70/40+) into a LOCAL module. @step extraction regex: `@step\s+(Given|When|Then|And|But)\s+(.+?)(?:\s*\*\/.*)?$` plus plain `^//\s+(Given|When|Then|And|But)\s+(.+)$`. detect_work_unit_type reads the @WORK-UNIT-ID tag (`@([A-Z]+-\d+)`) then work-units.json workUnits[id].type, defaulting to 'story'. Sidecar resolved at spec/features/<name>.feature.coverage (tolerate trailing .feature); writes via io::locked_file::write_json_atomic (2-space JSON, no trailing newline); extra-flatten preserves unknown fields.
  Modes: test-only (--test-file + --test-lines), impl-only (--test-file + --impl-file + --impl-lines), both (all four). Flag-combination, file-existence (skip-validation warnings), missing-sidecar (system-reminder), scenario-not-found and step-validation error messages all mirror TS verbatim. Returns envelope {success, message, warnings?}.
  Two-front-doors: dispatcher and clap CLI both call link_coverage::run. CLI bridge rust/fspec/src/link_coverage.rs marshals positional feature-name + --scenario/--test-file/--test-lines/--impl-file/--impl-lines/--skip-validation/--skip-step-validation into JSON only. SHARED-FILE REQUEST (supervisor): move link-coverage into run_ported and change the dispatch arm signature from run(args_json) to run(args_json, project_root); add to PORTED_COMMANDS in canonical.rs.
  """

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to link test and implementation files to a feature scenario's coverage sidecar with mandatory step-comment validation
    So that I maintain scenario-to-test-to-code traceability sharing one Rust source of truth between the LLM dispatcher and the CLI

  Scenario: test-only mode appends a test mapping and recalculates stats
    Given a temp project root has a feature file and matching coverage sidecar with scenario "Login", and a test file containing @step comments matching the scenario steps
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    Then the dispatcher returns success=true
    And the scenario "Login" gains a test mapping referencing that test file and line range
    And the stats coveredScenarios and coveragePercent increase accordingly
    And the result message contains "Linked test mapping"

  Scenario: impl-only mode adds an implementation mapping to an existing test mapping
    Given a temp project root has a coverage sidecar where scenario "Login" already has a test mapping for the test file
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, implFile and implLines "10-12"
    Then the dispatcher returns success=true
    And the test mapping gains an implementation mapping with lines [10, 11, 12]
    And the result message contains "implementation mapping"

  Scenario: both mode appends a test mapping carrying its implementation mapping
    Given a temp project root has a feature file and matching coverage sidecar with scenario "Login", and a test file with matching @step comments
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, testLines, implFile and implLines
    Then the dispatcher returns success=true
    And the scenario gains a test mapping whose implMappings includes the implementation file

  Scenario: impl-file without test-file surfaces a flag-combination error
    Given a temp project root has a coverage sidecar with scenario "Login"
    When I dispatch link-coverage for feature "user-login" with scenario="Login" and implFile only
    Then the dispatcher returns an error whose message contains "--test-file is required when adding implementation mappings"

  Scenario: test-file without test-lines surfaces a flag-combination error
    Given a temp project root has a coverage sidecar with scenario "Login"
    When I dispatch link-coverage for feature "user-login" with scenario="Login" and testFile only
    Then the dispatcher returns an error whose message contains "--test-lines is required when linking test file"

  Scenario: A missing test file without skip-validation errors
    Given a temp project root has a coverage sidecar with scenario "Login" and no test file on disk
    When I dispatch link-coverage for feature "user-login" with scenario="Login", a non-existent testFile and testLines
    Then the dispatcher returns an error whose message contains "File not found"

  Scenario: skip-validation downgrades a missing file to a warning
    Given a temp project root has a feature file and coverage sidecar with scenario "Login" tagged as a task work unit, and no test file on disk
    When I dispatch link-coverage for feature "user-login" with scenario="Login", a non-existent testFile, testLines, skipValidation true and skipStepValidation true
    Then the dispatcher returns success=true
    And the result warnings contain "validation skipped"

  Scenario: A missing coverage sidecar errors with a generate-coverage suggestion
    Given a temp project root has a feature file with scenarios but no coverage sidecar
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    Then the dispatcher returns an error whose message contains "Coverage file not found"
    And the error message suggests running fspec generate-coverage

  Scenario: A scenario absent from the sidecar errors with available scenarios listed
    Given a temp project root has a coverage sidecar that does not contain scenario "Nope"
    When I dispatch link-coverage for feature "user-login" with scenario="Nope", testFile and testLines
    Then the dispatcher returns an error whose message contains "Scenario not found"
    And the error message lists the available scenarios

  Scenario: Step validation fails when a required step comment is missing
    Given a temp project root has a story feature file whose scenario "Login" has steps, a coverage sidecar, and a test file missing one required @step comment
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    Then the dispatcher returns an error whose message contains "STEP VALIDATION FAILED"
    And the sidecar is not modified

  Scenario: skip-step-validation is rejected for a story work unit
    Given a temp project root has a story feature file for scenario "Login", a coverage sidecar, and a test file missing required @step comments
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, testLines and skipStepValidation true
    Then the dispatcher returns an error whose message contains "STEP VALIDATION ENFORCEMENT VIOLATION"
    And the sidecar is not modified

  Scenario: Written sidecar preserves unknown top-level fields and is atomic 2-space JSON
    Given a temp project root has a coverage sidecar carrying an unknown top-level field and scenario "Login" with a matching test file
    When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    Then the dispatcher returns success=true
    And the rewritten sidecar still contains the unknown top-level field
