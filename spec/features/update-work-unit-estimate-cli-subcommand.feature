@done
@mutation
@cli
@rust
@RPC-318
Feature: Port update-work-unit-estimate command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/update_work_unit_estimate.rs; signature pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>. Reads work-units via read_work_units_or_empty (ENOENT→empty, mirrors TS fileManager.readJSON default). FIBONACCI = [1,2,3,5,8,13,21]. Writes spec/work-units.json atomically via io::locked_file::write_json_atomic. estimate stored in WorkUnit.extra; updated_at typed.
  Prefill detection ported as a PRIVATE helper module inside the command file (NOT added to io/ensure.rs without supervisor approval). Mirrors src/utils/prefill-detection.ts: flat readdir of spec/features/*.feature, hand-rolled tag matcher for (^|\s)@<id>(?:\s|$), and the 9 prefill patterns scanned line-by-line (no regex crate). Returns None when spec/features missing or no tagged file found.
  The two ACDD-violation system-reminder blocks are byte-exact ports of the TS template literals (trimmed), wrapped by the outer 'Failed to update work unit estimate: ' prefix. CLI bridge at codelet/fspec/src/update_work_unit_estimate.rs marshals positional workUnitId + estimate (number) into JSON; prints '✓ Work unit <id> estimate set to <n>' on success, error to stderr on failure. Help config mirrors update-work-unit-estimate-help.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Both the LLM dispatcher and the clap CLI subcommand call the single commands::update_work_unit_estimate::run(args_json, project_root) function (two-front-doors)
  #   2. The estimate must be a Fibonacci number from {1,2,3,5,8,13,21}; otherwise return error 'Failed to update work unit estimate: Invalid estimate: <n>. Must be one of: 1,2,3,5,8,13,21'
  #   3. If the work unit does not exist, return error 'Failed to update work unit estimate: Work unit <id> not found'
  #   4. For story/bug/untyped work units, a feature file tagged @<id> must exist; if none is found, return the ACDD-violation system-reminder block (wrapped with the 'Failed to update work unit estimate: ' prefix)
  #   5. For story/bug/untyped work units, if the tagged feature file contains the role/action/benefit/precondition/expected-outcome/scenario-name bracket tokens, TODO markers, or component/feature-group tag placeholders, return the prefill ACDD-violation system-reminder block listing up to 3 matches
  #   6. Task work units are EXEMPT from the feature-file/prefill gate and can be estimated without any feature file
  #   7. On success, set the estimate and updatedAt on the work unit, write spec/work-units.json atomically, and return { success: true }
  #   8. All thrown errors are wrapped with the prefix 'Failed to update work unit estimate: '
  #   9. fspec update-work-unit-estimate --help is byte-for-byte identical to node dist/index.js update-work-unit-estimate --help
  #
  # EXAMPLES:
  #   1. Dispatch update-work-unit-estimate TASK-001 with estimate=3 succeeds without any feature file (task exempt)
  #   2. Dispatch update-work-unit-estimate AUTH-001 with estimate=7 returns success=false with 'Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21'
  #   3. Dispatch update-work-unit-estimate MISSING-999 with estimate=5 returns success=false with 'Work unit MISSING-999 not found'
  #   4. Dispatch update-work-unit-estimate AUTH-001 (story, no feature file) returns success=false with the ACDD-violation 'without completed feature file' message
  #   5. Dispatch update-work-unit-estimate AUTH-001 (story, feature file tagged @AUTH-001 containing a role-placeholder token) returns success=false with the prefill ACDD-violation message
  #   6. Dispatch update-work-unit-estimate AUTH-001 (story, clean feature file tagged @AUTH-001) with estimate=5 succeeds and sets estimate=5
  #   7. CLI: ./fspec update-work-unit-estimate TASK-001 3 exits 0 and prints '✓ Work unit TASK-001 estimate set to 3'
  #   8. CLI: ./fspec update-work-unit-estimate TASK-001 7 exits 1 and writes the invalid-estimate failure to stderr
  #
  # ========================================

  Background: User Story
    As a fspec maintainer
    I want to port the update-work-unit-estimate command to the Rust fspec-core crate
    So that the standalone fspec binary can set Fibonacci story-point estimates natively with the same ACDD prefill gate as TypeScript

  Scenario: Clap exposes update-work-unit-estimate with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec update-work-unit-estimate --help`
    Then the command exits 0
    And stdout describes the update-work-unit-estimate subcommand
    And stdout mentions the `<id>` argument
    And stdout mentions the `<points>` argument
    And the --help output is byte-for-byte identical to the captured TS reference fixture

  Scenario: CLI sets a task estimate and prints the success line
    Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    When I run `./codelet/target/release/fspec update-work-unit-estimate TASK-001 3`
    Then the command exits 0
    And stdout contains the line '✓ Work unit TASK-001 estimate set to 3'
    And spec/work-units.json work unit 'TASK-001' has estimate 3

  Scenario: CLI reports an invalid estimate on stderr
    Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    When I run `./codelet/target/release/fspec update-work-unit-estimate TASK-001 7`
    Then the command exits 1
    And stderr contains the substring 'Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21'
