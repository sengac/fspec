@done
@cli
@quality-assurance
@development-tools
@BUG-565
Feature: The quality checks lint every workspace test target
  """
  The CLI helper saw 'cargo clippy -p codelet-fspec --tests -- -D warnings' fail on an unused import in rust/fspec/tests/rpc070_create_session_no_panic.rs. Re-checked 2026-11-07 at 1d6abb631: 'cargo clippy --workspace --tests -- -D warnings' (with and without --all-features) is clean, so the import is gone; the gap was that the documented quality checks only ran 'cargo clippy -p codelet-fspec', which never lints test targets. AGENTS.md 'Quality Check Integration' now lists 'cargo clippy --workspace --tests -- -D warnings', and the local spec/fspec-config.json runs it too (fspec configure-tools). Tests: rust/fspec/tests/bug565_workspace_clippy_tests_quality_check.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The quality checks in AGENTS.md (section 'Quality Check Integration') include the exact command 'cargo clippy --workspace --tests -- -D warnings', so every test target of every workspace crate is linted, not only codelet-fspec's library code
  #   2. The workspace clippy --tests run is clean: re-checked 2026-11-07 at 1d6abb631 (with and without --all-features), the unused import in rust/fspec/tests/rpc070_create_session_no_panic.rs no longer reproduces, so no source change is needed; the gap was that nothing ran it
  #   3. The local spec/fspec-config.json qualityCheck commands (gitignored, set with fspec configure-tools) also run the workspace clippy --tests check
  #
  # EXAMPLES:
  #   1. AGENTS.md 'Quality Check Integration' lists 'cargo clippy --workspace --tests -- -D warnings'; a test reads the block and finds the exact line
  #   2. A test file with an unused import makes 'cargo clippy --workspace --tests -- -D warnings' exit non-zero, so a maintainer following the quality checks sees it before committing
  #
  # ========================================
  Background: User Story
    As a maintainer committing Rust changes
    I want to have the documented quality checks lint every workspace test target with clippy -D warnings
    So that an unused import in a test file cannot land unnoticed

  Scenario: The documented quality checks lint every workspace test target
    Given the "Quality Check Integration" section of AGENTS.md
    When a maintainer reads the quality checks to run before committing
    Then the section lists "cargo clippy --workspace --tests -- -D warnings"
