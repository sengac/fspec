@done
@rust
@coverage-tracking
@cli
@RPC-197
Feature: audit-coverage CLI subcommand
  """
  Core impl: rewrite rust/fspec-core/src/commands/audit_coverage.rs to `pub async fn run(args_json, project_root)`; reuse crate::types::coverage::CoverageFile (no new shared type).
  Core returns JSON envelope {output, exitCode} (validate.rs pattern); CLI bridge rust/fspec/src/audit_coverage.rs parses it, prints output, returns exitCode. Help config rust/fspec-core/src/help/configs/audit_coverage.rs. SUPERVISOR must wire: canonical PORTED_COMMANDS, dispatch run_ported, main.rs Mode::AuditCoverage + intercept + mod, help configs/mod.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Resolves spec/features/<feature-name>.feature.coverage relative to project_root (raw name, no .feature stripping)
  #   2. When the coverage file is missing, returns output '✗ Coverage file not found: <absolutePath>' with exitCode 1
  #   3. Collects every testMapping.file and implMapping.file into allFiles; flags those whose project_root-relative path does not exist as missingFiles tagged test or implementation
  #   4. When no files are missing, output is '✅ All files found (n/n)' newline 'All mappings valid' with exitCode 0
  #   5. When files are missing, output header '✗ m missing file(s) out of n total files' then a 3-line block per missing file ('❌ <Test|Implementation> file not found: <file>' + recommendation), with exitCode 1
  #   6. The clap subcommand takes a required positional <feature-name> and registers NO options (Framing A: the --fix flag in the help doc is not implemented by the real TS CLI)
  #   7. Both front doors (LLM dispatcher and standalone CLI) call the single fspec_core::commands::audit_coverage::run function
  #
  # EXAMPLES:
  #   1. Given a coverage file referencing 3 files that all exist, when audit-coverage runs, output contains '✅ All files found (3/3)' and 'All mappings valid', exit 0
  #   2. Given a coverage file mapping to a deleted test file, when audit-coverage runs, output shows '❌ Test file not found: src/__tests__/deleted.test.ts' plus the recommendation line, exit 1
  #   3. Given no coverage file exists for the feature, when audit-coverage runs, output is '✗ Coverage file not found: <path>' and exit 1
  #   4. Given the binary is run with 'audit-coverage --help', output is byte-for-byte identical to the captured TS formatCommandHelp fixture, exit 0
  #
  # ========================================
  Background: User Story
    As a developer porting fspec to Rust
    I want to run audit-coverage through both the LLM dispatcher and the standalone Rust CLI
    So that coverage-file file-existence auditing has byte-parity with the TypeScript implementation

  Scenario: Clap exposes audit-coverage as a subcommand requiring a feature-name and printing byte-parity help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec audit-coverage --help` piped to non-TTY
    Then the command exits 0
    Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/audit-coverage.txt
    Then stdout starts with a blank line followed by 'AUDIT-COVERAGE'

  Scenario: CLI reports all files present and exits 0
    Given a project root whose spec/features/user-login.feature.coverage references three files that all exist
    When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    Then the command exits 0
    Then stdout contains the substring '✅ All files found (3/3)'
    Then stdout contains the substring 'All mappings valid'

  Scenario: CLI reports a missing test file and exits 1
    Given a project root whose spec/features/user-login.feature.coverage maps to a test file that does not exist
    When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    Then the command exits 1
    Then stdout contains the substring '❌ Test file not found:'
    Then stdout contains the substring 'Recommendation: Remove this mapping or restore the deleted file'

  Scenario: CLI reports a missing coverage file and exits 1
    Given a project root with no spec/features/user-login.feature.coverage file
    When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    Then the command exits 1
    Then stdout contains the substring '✗ Coverage file not found:'

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given a project root whose spec/features/user-login.feature.coverage references files that all exist
    When I dispatch audit-coverage through fspec_core::dispatch::dispatch_command for feature 'user-login'
    Then the dispatcher's DispatchResult.data carries the same output and exitCode the CLI prints against the same on-disk state
    Then the CLI bridge module rust/fspec/src/audit_coverage.rs contains NO inline file-existence or rendering logic — its only computation is JSON arg marshalling and envelope decoding
