@BUG-582
@done
@BUG-383
@tui
@test-maintenance
@rust
Feature: The TUI test harness reports stuck and failed tasks
  """
  rust/fspec-tui/tests/common/harness/driving.rs: drain_pending wrapped the
  drain in a 2 s timeout whose Err was discarded (let _ = timeout(..)) and
  discarded every JoinHandle result (let _ = handle.await); wait_for_mock and
  wait_for_self dropped join results too. A stuck or panicking spawned task
  let a test continue on partial state. drain_pending, wait_for_mock and
  wait_for_self now return Result<(), String> naming the timeout or the
  task's JoinError, and call sites propagate it. App::track_pending_task is
  the test seam that queues a task (the counterpart of next_pending_task).
  The include! composition of MockBackend is REFAC-062.
  TEST-042: the 2 s drain timeout is now the deadlock guard
  (codelet_test_helpers::deadlock_guard), so a loaded machine never fails a
  drain; a stuck task keeps the drain pending until the guard fails it loudly.
  Tests: rust/fspec-tui/tests/bug383_tui_harness_reports_stuck_tasks.rs.
  """

  Background: User Story
    As a developer writing fspec-tui integration tests
    I want to have the harness fail when a spawned task hangs or panics
    So that a test never passes on half-drained or partial app state

  Scenario: A panicking spawned task fails the drain
    Given a harness with a pending task that panics
    When the pending work is drained
    Then the drain fails naming the failed task

  Scenario: A spawned task that never finishes fails the drain
    Given a harness with a pending task that never finishes
    When the pending work is drained
    Then the drain stays pending while the task is stuck, for the deadlock guard to fail it
