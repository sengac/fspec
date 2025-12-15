@scaffold
@provider-management
@REFAC-002
Feature: Add rig-core dependency and compatibility layer

  """
  Add rig-core 0.25.0 to Cargo.toml. Re-export rig types from src/lib.rs for future use. No changes to existing code. Foundation for REFAC-003 (provider refactor) and REFAC-004 (agent refactor). Must pass all existing tests and clippy without warnings.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must add rig-core 0.25.0 to Cargo.toml dependencies
  #   2. All existing tests must continue to pass without modification
  #   3. No changes to public API - existing code must compile unchanged
  #   4. Must re-export rig types from src/lib.rs for future use
  #   5. Cargo build and cargo clippy must complete without warnings
  #
  # EXAMPLES:
  #   1. Developer adds rig-core = 0.25.0 to Cargo.toml, runs cargo build, compilation succeeds
  #   2. Developer runs cargo test after adding rig-core, all 10 existing integration tests pass
  #   3. Developer adds pub use rig to src/lib.rs, can import rig::completion::CompletionModel in test file
  #   4. Developer runs cargo clippy -- -D warnings, completes with zero warnings
  #
  # ========================================

  Background: User Story
    As a developer working on codelet
    I want to add rig-core dependency without breaking existing functionality
    So that I can incrementally migrate to rig's streaming and multi-provider support

  Scenario: Add rig-core dependency to Cargo.toml
    Given I have a Cargo.toml file in the project root
    When I add "rig-core = \"0.25.0\"" to the dependencies section
    And I run "cargo build"
    Then the build should succeed without errors
    And rig-core 0.25.0 should be downloaded and compiled

  Scenario: All existing tests pass after adding rig-core
    Given rig-core 0.25.0 is added to Cargo.toml
    And the project builds successfully
    When I run "cargo test"
    Then all 10 existing integration tests should pass
    And no test failures should occur
    And test output should show "test result: ok"

  Scenario: Re-export rig types from lib.rs
    Given rig-core 0.25.0 is added to Cargo.toml
    When I add "pub use rig;" to src/lib.rs
    And I create a test file that imports "use codelet::rig::completion::CompletionModel;"
    Then the test file should compile without errors
    And rig types should be accessible from the codelet namespace

  Scenario: Cargo clippy completes without warnings
    Given rig-core 0.25.0 is added and re-exported
    And all code changes are complete
    When I run "cargo clippy -- -D warnings"
    Then clippy should complete with exit code 0
    And no warnings should be reported
    And the output should confirm "0 warnings emitted"
