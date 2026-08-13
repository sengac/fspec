@done
@rust
@validation
@cli
@RPC-201
Feature: check CLI subcommand
  """
  Core impl: rewrite rust/fspec-core/src/commands/check.rs to `pub async fn run(args_json, project_root)`; reuse io::feature_glob::glob_feature_files, io::gherkin::parse_feature_lenient, and call commands::validate_tags::run internally (await it). Returns full JSON result object {success, gherkinStatus, tagStatus, formatStatus, fileCount, errors?, message?, details?}.
  BLOCKING DECISION for SUPERVISOR: formatStatus needs a Gherkin AST->text formatter (src/utils/gherkin-formatter.ts ~380 LOC) that does NOT exist in Rust (format RPC-230 still a stub). Option A: port formatter to shared io::gherkin_format.rs first (cross-worker dep). Option B (recommended): land check now with gherkin+tag fully ported and formatStatus=SKIP until the formatter lands (SKIP never fails success, matching TS outer-catch behaviour). CLI bridge rust/fspec/src/check.rs renders display + exit code. SUPERVISOR must wire: canonical PORTED_COMMANDS, dispatch run_ported, main.rs Mode::Check{verbose} + intercept + mod, help configs/mod.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When no feature files exist under spec/features, returns success=true with message 'No feature files found' and fileCount 0
  #   2. Runs three sub-checks: Gherkin syntax (parse each feature file), tag validation (delegates to validate_tags::run), and formatting (parse + reformat + compare); each yields PASS/FAIL/SKIP
  #   3. success is true only when no sub-check is FAIL (SKIP does not fail success); exit code is 0 on success, 1 otherwise
  #   4. Gherkin syntax errors push 'Gherkin syntax error in <file>: <message>'; tag failures push each per-tag error.message; formatting failures push 'Formatting check failed: <file> needs formatting'
  #   5. CLI renders 'Running validation checks...', an optional 'Checked N feature file(s)' line (only when fileCount>0), the three 'X: PASS|FAIL|SKIP' lines, an Errors list when present, then '✓ All checks passed' or '✗ Some checks failed'
  #   6. Formatting sub-check requires a Gherkin AST formatter that does not yet exist in Rust; pending that module it reports SKIP (a legitimate non-failing state per TS) — tracked divergence awaiting supervisor decision A (port formatter first) vs B (SKIP for now)
  #   7. The clap subcommand exposes -v/--verbose (default false) and NO positional args; both front doors call the single fspec_core::commands::check::run function
  #
  # EXAMPLES:
  #   1. Given three valid feature files with registered tags, when check runs, gherkin and tag statuses are PASS and success=true, exit 0
  #   2. Given a feature file with invalid Gherkin syntax, when check runs, gherkin status is FAIL, success=false, exit 1, and an error mentioning the file is reported
  #   3. Given a feature file with an unregistered tag '@unknown-tag', when check runs, tag status is FAIL, success=false, exit 1, and the unregistered tag appears in the errors
  #   4. Given no feature files exist, when check runs, the output reports 'No feature files found' and exits 0
  #   5. Given the binary is run with 'check --help', output is byte-for-byte identical to the captured TS formatCommandHelp fixture, exit 0
  #
  # ========================================
  Background: User Story
    As a developer porting fspec to Rust
    I want to run check through the LLM dispatcher and the standalone Rust CLI
    So that the combined Gherkin-syntax, tag, and formatting validation has parity with the TypeScript implementation

  Scenario: Clap exposes check with -v/--verbose and prints byte-parity help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec check --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/check.txt
    Then stdout starts with a blank line followed by 'CHECK'

  Scenario: CLI passes and exits 0 for valid registered feature files
    Given a project root whose spec/features holds valid feature files with registered tags
    When I run `./rust/target/release/fspec check` from that directory
    Then the command exits 0
    Then stdout contains the substring 'Gherkin syntax: PASS'
    Then stdout contains the substring 'Tag validation: PASS'
    Then stdout contains the substring 'All checks passed'

  Scenario: CLI exits 1 when a feature file has invalid Gherkin syntax
    Given a project root whose spec/features holds a feature file with invalid Gherkin syntax
    When I run `./rust/target/release/fspec check` from that directory
    Then the command exits 1
    Then stdout contains the substring 'Gherkin syntax: FAIL'
    Then stdout contains the substring 'Some checks failed'

  Scenario: CLI reports the no-files case and exits 0
    Given a project root with no feature files under spec/features
    When I run `./rust/target/release/fspec check` from that directory
    Then the command exits 0

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/features holds valid feature files with registered tags
    When I dispatch check through fspec_core::dispatch::dispatch_command
    Then the dispatcher's DispatchResult.data reports the same gherkinStatus and tagStatus the CLI renders against the same on-disk state
    Then the CLI bridge module rust/fspec/src/check.rs contains NO inline parsing, tag-validation, or check-aggregation logic — its only computation is JSON arg marshalling and display rendering from the envelope
