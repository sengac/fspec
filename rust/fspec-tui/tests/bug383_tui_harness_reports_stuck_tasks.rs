//! BUG-383 — the TUI test harness reports stuck and failed tasks.
//!
//! Feature: spec/features/tui-harness-reports-stuck-tasks.feature

mod common;

use codelet_test_helpers::TestResult;
use common::harness::AppTestHarness;

#[tokio::test]
async fn a_panicking_spawned_task_fails_the_drain() -> TestResult {
    // @step Given a harness with a pending task that panics
    let mut harness = AppTestHarness::new();
    harness.app.track_pending_task(tokio::spawn(async {
        std::panic::resume_unwind(Box::new("BUG-383 scripted task failure"))
    }));
    // @step When the pending work is drained
    let drained = harness.drain_pending().await;
    // @step Then the drain fails naming the failed task
    let error = drained.err().unwrap_or_default();
    assert!(error.contains("panicked"), "{error}");
    Ok(())
}

#[tokio::test]
async fn a_spawned_task_that_never_finishes_fails_the_drain() -> TestResult {
    // @step Given a harness with a pending task that never finishes
    let mut harness = AppTestHarness::new();
    harness
        .app
        .track_pending_task(tokio::spawn(std::future::pending::<()>()));
    // @step When the pending work is drained
    let mut drain = Box::pin(harness.drain_pending());
    // @step Then the drain stays pending while the task is stuck, for the deadlock guard to fail it
    // TEST-042: no short timeout to wait out. The drain must still be
    // pending after many polls (a fixed number of rounds, not a window);
    // the deadlock guard inside it ends a real hang loudly.
    let mut polls = 0;
    codelet_test_helpers::pending_until("64 polls of the drain", &mut drain, || {
        polls += 1;
        polls > 64
    })
    .await?;
    Ok(())
}
