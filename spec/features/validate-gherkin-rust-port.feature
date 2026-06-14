@validation
@cli
@rust
@wip
@RPC-320
Feature: Port validate command to Rust

  """
  RPC-329 PARSER-DIVERGENCE ADDENDUM (Group-A parity review 2026-06-14): two further symptoms of the cucumber-gherkin vs gherkin-0.16 mismatch are known divergences, NOT defects in this card. (a) Orphan scenario (no Feature keyword): TS emits 'Suggestion: Add Feature keyword...' on 'Line 0'; Rust drops the Suggestion (getSuggestion keys off message text that gherkin-0.16 does not produce) and reports 'Line 1'. (b) Unescaped triple quotes inside a DocString: cucumber parses OK so TS runs the checkForCommonIssues heuristic and emits the canonical 'Unescaped triple quotes ... found inside DocString' message; gherkin-0.16 fails to parse the same bytes, so Rust takes the error branch and the heuristic (Ok-branch only, parity with TS) never runs. Both deferred to RPC-329. No dedicated triple-quote scenario was ever written here, so this path was never test-driven.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When no file argument is given the command MUST glob spec/features/**/*.feature (reuse io::feature_glob::glob_feature_files); if zero feature files are found it MUST emit 'No feature files found in spec/features/' to stderr and exit 2 (parity with src/commands/validate.ts:27-30)
  #   2. When a single file argument is given the command MUST validate only that file (resolved relative to project root); a missing file MUST produce a validation error 'File not found: <path>' on line 0 (parity with the ENOENT branch at src/commands/validate.ts:136-141)
  #   3. Each file is parsed with the gherkin parser (reuse io::gherkin::parse_feature_lenient); on parse error the result is valid=false with a single error {line, message, suggestion} where suggestion is derived from getSuggestion(message) heuristics (parity with src/commands/validate.ts:106-116)
  #   4. Beyond parser errors the command MUST run additional checks on the raw content: (a) unescaped triple-quotes inside a DocString → error 'Unescaped triple quotes (""") found inside DocString'; (b) more than 2 consecutive blank lines → error 'Excessive blank lines detected (N consecutive blank lines)' (parity with checkForCommonIssues at src/commands/validate.ts:166-227)
  #   5. The display loop prints '✓ <file> is valid' for valid files and '✗ <file> has syntax errors:' followed by '  Line N: <message>' and optional '  Suggestion: <s>' for invalid files; when more than one file is validated a summary line is appended: '✓ All N feature files are valid' (all valid) or 'Validated N files: X valid, Y invalid' (parity with src/commands/validate.ts:36-68)
  #   6. Exit codes: 0 when every validated file is valid; 1 when one or more files have syntax errors; 2 when no feature files are found OR an unexpected top-level error occurs (parity with process.exit calls at src/commands/validate.ts:29,72,76)
  #   7. The standalone fspec binary MUST expose 'validate' as a clap subcommand with an optional positional [file] argument and a -v/--verbose boolean flag (parity with the Commander.js registration at src/commands/validate.ts:256-265); the CLI bridge MUST delegate to the same fspec_core::commands::validate::run function (two front doors, one source of truth)
  #   8. Running `fspec validate --help` MUST print help byte-for-byte identical to the TS formatCommandHelp output captured from `node dist/index.js validate --help` piped to non-TTY (includes WHEN TO USE, ARGUMENTS, OPTIONS, EXAMPLES, COMMON ERRORS, TYPICAL WORKFLOW, RELATED COMMANDS, NOTES sections)
  #
  # EXAMPLES:
  #   1. Dispatch validate against a tempdir with two valid feature files → success, output lists '✓ <file> is valid' for each plus '✓ All 2 feature files are valid', exit 0
  #   2. Dispatch validate against a single valid file path → output is exactly '✓ <file> is valid' with no summary line (single-file path skips the >1 summary), exit 0
  #   3. Dispatch validate against a tempdir with one valid and one syntactically-broken file → output marks the broken file '✗ <file> has syntax errors:' with a 'Line N:' detail, summary 'Validated 2 files: 1 valid, 1 invalid', exit 1
  #   4. Dispatch validate against a tempdir with an empty spec/features/ (zero .feature files) → 'No feature files found in spec/features/' on stderr, exit 2
  #   5. Dispatch validate against a file containing 4 consecutive blank lines → valid=false with error message containing 'Excessive blank lines detected', exit 1 (content heuristic, independent of parser)
  #   6. Running `./codelet/target/release/fspec validate <single-valid-file>` prints '✓ <file> is valid' to stdout and exits 0; running it against a broken file exits 1 with the '✗ ... has syntax errors:' marker and a 'Line N:' detail on stdout
  #   7. Running `./codelet/target/release/fspec validate --help` prints the formatted help block byte-identical to the captured TS fixture and exits 0; the help lists the [file] argument and -v/--verbose option
  #
  # QUESTIONS (ANSWERED):
  #   Q: RPC-329 — the raw gherkin parser-error TEXT diverges from @cucumber/gherkin. Confirm tests should assert on structural facts + matching substrings only (NOT exact raw message), matching the sibling-command precedent (add_scenario.rs). Supervisor: OK to proceed with this framing without blocking?
  #   A: Yes — tests assert structural facts (file path, valid/invalid marker, exit code, Line N presence, Suggestion presence) and matching substrings only, NOT the exact raw parser-error text. This follows the sibling-command precedent (add_scenario.rs) and is consistent with the open RPC-329 bug. Content-heuristic messages (Unescaped triple quotes / Excessive blank lines) and getSuggestion lines ARE asserted exactly since they are parser-independent.
  #
  # ASSUMPTIONS:
  #   1. Yes — tests assert structural facts (file path, valid/invalid marker, exit code, Line N presence, Suggestion presence) and matching substrings only, NOT the exact raw parser-error text. This follows the sibling-command precedent (add_scenario.rs) and is consistent with the open RPC-329 bug. Content-heuristic messages (Unescaped triple quotes / Excessive blank lines) and getSuggestion lines ARE asserted exactly since they are parser-independent.
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to validate Gherkin syntax across spec/features/**/*.feature (or a single file) from both the LLM dispatcher and the shell CLI
    So that I can catch malformed feature files before moving from specifying to testing, without relying on Node.js

  Scenario: Validates all feature files and reports an all-valid summary
    Given spec/features/ contains two syntactically valid feature files
    When I dispatch the validate command against that project root with no file argument
    Then the dispatcher returns success=true
    Then the rendered output contains a '✓ <file> is valid' line for each file
    Then the rendered output contains the summary line '✓ All 2 feature files are valid'

  Scenario: Validates a single valid file with no summary line
    Given spec/features/login.feature is a syntactically valid feature file
    When I dispatch the validate command against that project root with the file argument 'spec/features/login.feature'
    Then the dispatcher returns success=true
    Then the rendered output is exactly '✓ spec/features/login.feature is valid'
    Then the rendered output does NOT contain the substring 'feature files are valid'

  Scenario: Marks a syntactically broken file and reports a mixed summary
    Given spec/features/ contains one valid file and one file with broken Gherkin syntax
    When I dispatch the validate command against that project root with no file argument
    Then the rendered output contains the substring '✗' followed by ' has syntax errors:'
    Then the rendered output contains a 'Line ' detail for the broken file
    Then the rendered output contains the summary line 'Validated 2 files: 1 valid, 1 invalid'
    Then the dispatcher result reports an exit code of 1

  Scenario: Reports no feature files found when spec/features is empty
    Given spec/features/ exists but contains zero .feature files
    When I dispatch the validate command against that project root with no file argument
    Then the dispatcher returns an error containing the substring 'No feature files found in spec/features/'
    Then the dispatcher result reports an exit code of 2

  Scenario: Flags excessive consecutive blank lines via the content heuristic
    Given a feature file containing four consecutive blank lines
    When I dispatch the validate command against that single file
    Then the rendered output marks the file invalid
    Then the rendered output contains the substring 'Excessive blank lines detected'
    Then the dispatcher result reports an exit code of 1
