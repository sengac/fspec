@BUG-582
@done
@BUG-406
@utils
@test-maintenance
@rust
Feature: Test binaries share one live data directory
  """
  codelet_common::set_data_directory sets a process-global path. Test files
  called it as `let _ = ..set_data_directory(..)`, dropping its Result, and
  several passed a per-test tempdir that was deleted when that test ended
  while other tests in the same binary still used the global path (the
  BUG-397 failure mode: tests fail together or by order, pass alone).
  codelet_test_helpers::shared_temp_directory() now returns one TempDir per
  process (OnceLock); every call site sets the data directory to it and
  handles the Result.
  Tests: rust/test-helpers/tests/bug406_test_shared_data_directory.rs.
  """

  Background: User Story
    As a developer running a test binary
    I want to have every test share one live data directory and see a failed set_data_directory
    So that tests never fail together or by order because another test's tempdir was deleted

  Scenario: The shared temp directory is the same live directory for every caller
    Given a test binary
    When two callers ask for the shared temp directory
    Then both get the same directory and it exists

  Scenario: No test drops the set_data_directory result
    Given the workspace test sources
    When they are scanned for set_data_directory calls
    Then none discards the result with let _ =
