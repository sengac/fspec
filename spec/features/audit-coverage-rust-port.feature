@done
@rust
@coverage-tracking
@cli
@RPC-197
Feature: Port audit-coverage command to Rust
  """
  Rewrite the stub at rust/fspec-core/src/commands/audit_coverage.rs to
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
  Reuse crate::types::coverage::CoverageFile (already exists for show-coverage RPC-300) —
  no new shared type. Core returns a JSON envelope {output, exitCode} (validate.rs pattern).
  The CLI bridge (rust/fspec/src/audit_coverage.rs) parses the envelope, prints output,
  and returns exitCode.

  Framing A divergence: audit-coverage-help.ts documents a --fix flag and per-scenario
  output that the real TS auditCoverage implementation does NOT produce. We port the
  ACTUAL implementation output (✅ All files found / ❌ file not found), NOT the help-doc
  aspirational examples. The --help fixture is captured verbatim from the TS help doc.
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
  #   5. When files are missing, output header '✗ m missing file(s) out of n total files' then a 3-line block per missing file with exitCode 1
  #   6. Both front doors call the single fspec_core::commands::audit_coverage::run function
  #
  # ========================================
  Background: User Story
    As a developer porting fspec to Rust
    I want to run audit-coverage through the LLM dispatcher
    So that coverage-file file-existence auditing has byte-parity with the TypeScript implementation

  Scenario: Reports all files present when every referenced file exists
    Given a coverage file user-login.feature.coverage referencing three files that all exist on disk
    When I dispatch the audit-coverage command for feature 'user-login' against that project root
    Then the dispatcher returns success=true
    Then the output contains the substring '✅ All files found (3/3)'
    Then the output contains the substring 'All mappings valid'
    Then the envelope exitCode is 0

  Scenario: Detects a missing test file and recommends removing the mapping
    Given a coverage file user-login.feature.coverage mapping to the test file 'src/__tests__/deleted.test.ts' which does not exist
    When I dispatch the audit-coverage command for feature 'user-login' against that project root
    Then the output contains the substring '❌ Test file not found: src/__tests__/deleted.test.ts'
    Then the output contains the substring 'Recommendation: Remove this mapping or restore the deleted file'
    Then the envelope exitCode is 1

  Scenario: Detects a missing implementation file
    Given a coverage file user-login.feature.coverage whose test file exists but maps to the implementation file 'src/auth/deleted.ts' which does not exist
    When I dispatch the audit-coverage command for feature 'user-login' against that project root
    Then the output contains the substring '❌ Implementation file not found: src/auth/deleted.ts'
    Then the envelope exitCode is 1

  Scenario: Reports a missing coverage file with the full path
    Given a project root with no spec/features/user-login.feature.coverage file
    When I dispatch the audit-coverage command for feature 'user-login' against that project root
    Then the output contains the substring '✗ Coverage file not found:'
    Then the envelope exitCode is 1
