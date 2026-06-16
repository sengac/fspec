@done
@rust
@validation
@cli
@RPC-201
Feature: Port check command to Rust

  """
  Core impl: rewrite codelet/fspec-core/src/commands/check.rs to `pub async fn run(args_json, project_root)`; reuse io::feature_glob::glob_feature_files, io::gherkin::parse_feature_lenient, and call commands::validate_tags::run internally (await it). Returns full JSON result object {success, gherkinStatus, tagStatus, formatStatus, fileCount, errors?, message?, details?}.
  BLOCKING DECISION for SUPERVISOR: formatStatus needs a Gherkin AST->text formatter (src/utils/gherkin-formatter.ts ~380 LOC) that does NOT exist in Rust (format RPC-230 still a stub). Option A: port formatter to shared io::gherkin_format.rs first (cross-worker dep). Option B (recommended): land check now with gherkin+tag fully ported and formatStatus=SKIP until the formatter lands (SKIP never fails success, matching TS outer-catch behaviour). CLI bridge codelet/fspec/src/check.rs renders display + exit code. SUPERVISOR must wire: canonical PORTED_COMMANDS, dispatch run_ported, main.rs Mode::Check{verbose} + intercept + mod, help configs/mod.rs.
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

  Scenario: All sub-checks pass for valid registered feature files
    Given spec/features contains three valid feature files whose tags are all registered in spec/tags.json
    When I dispatch the check command against that project root
    Then the dispatcher returns success=true
    Then the gherkinStatus is 'PASS'
    Then the tagStatus is 'PASS'

  Scenario: Gherkin syntax failure fails the check
    Given spec/features contains a feature file with invalid Gherkin syntax
    When I dispatch the check command against that project root
    Then the gherkinStatus is 'FAIL'
    Then the result success field is false
    Then the errors list contains an entry mentioning that file

  Scenario: An unregistered tag fails the check
    Given spec/features contains a valid feature file carrying the unregistered tag '@unknown-tag'
    When I dispatch the check command against that project root
    Then the tagStatus is 'FAIL'
    Then the result success field is false
    Then the errors list mentions '@unknown-tag'

  Scenario: No feature files reports the canonical message and succeeds
    Given a project root with no feature files under spec/features
    When I dispatch the check command against that project root
    Then the dispatcher returns success=true
    Then the result message is 'No feature files found'
    Then the fileCount is 0
