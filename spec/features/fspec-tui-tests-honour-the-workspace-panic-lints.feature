@done
@BUG-471
@BUG-366
@tui
@test-maintenance
@rust
Feature: fspec-tui tests honour the workspace unwrap, expect and panic lints
  """
  The workspace denies clippy::unwrap_used, expect_used and panic, but most
  rust/fspec-tui/tests files and the shared tests/common module opted out
  with #![allow(...)], and helpers dropped task join errors with
  `let _ = handle.await`. Tests now return TestResult and propagate failures
  with `?` (codelet_test_helpers::{OrFail, OrFailErr, TestResult}); a
  deliberate panic is allowed only by a scoped, reasoned #[allow].
  Tests: rust/fspec-tui/tests/bug366_tests_honour_panic_lints.rs.
  """

  Background: User Story
    As a developer relying on the fspec-tui test suite
    I want tests to fail through Result instead of unwrap, expect and panic
    So that the workspace lints hold everywhere and no failure is silently dropped

  Scenario: No fspec-tui test file or helper module suppresses the lints
    Given every Rust file under rust/fspec-tui/tests
    When their file and module level allow attributes are read
    Then none allows clippy::unwrap_used, clippy::expect_used or clippy::panic

  Scenario: A deliberate panic carries a scoped, reasoned allow
    Given every Rust file under rust/fspec-tui/tests
    When an item-level allow of clippy::panic is found
    Then it states its reason

  Scenario: No fspec-tui test drops a task's join error
    Given every Rust file under rust/fspec-tui/tests
    When awaited tasks are inspected
    Then none is awaited as `let _ = <task>.await`

  Scenario: A task the App cancelled does not fail a test
    Given every Rust file under rust/fspec-tui/tests
    When the App's pending tasks are drained
    Then each join goes through completed_or_cancelled, so a task the App aborted passes and a panicked task fails

  Scenario: No fspec-tui test recovers a poisoned lock
    Given every Rust file under rust/fspec-tui/tests
    When their lock calls are inspected
    Then none recovers a poisoned lock with PoisonError::into_inner
