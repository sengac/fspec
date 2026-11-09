//! BUG-379 — a failed work-unit or checkpoint-count refetch after a lagged
//! work-unit broadcast or a daemon switch is shown as an error instead of
//! leaving a silently stale board.
//!
//! Feature: spec/features/work-units-refetch-errors.feature

use codelet_test_helpers::TestResult;
use std::sync::Arc;

use codelet_fspec_tui::daemons::SwitchedBackend;
use codelet_fspec_tui::Action;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::harness::AppTestHarness;
use common::MockBackend;

const DIALOG_ID: &str = "session-storage-dialog";

/// More snapshots than the mock's 64-slot work-units broadcast holds.
const OVERFLOWING_SNAPSHOTS: usize = 80;

/// The rendered screen, one line per row.
fn screen(harness: &mut AppTestHarness) -> String {
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ok when the error dialog is open and shows `expected`.
fn expect_error_dialog(harness: &mut AppTestHarness, expected: &str) -> TestResult {
    if !harness.compositor_contains(DIALOG_ID) {
        return Err(format!("no error dialog is shown; expected {expected:?}").into());
    }
    let screen = screen(harness);
    if !screen.contains(expected) {
        return Err(format!("{expected:?} is not shown in:\n{screen}").into());
    }
    Ok(())
}

/// Switch the harness app to a daemon named `name` served by `backend`.
async fn switch_daemon(
    harness: &mut AppTestHarness,
    name: &str,
    backend: Arc<MockBackend>,
) -> Result<(), String> {
    harness
        .app
        .dispatch(Action::DaemonSwitched(SwitchedBackend {
            name: name.into(),
            backend,
        }));
    harness.drain_pending().await
}

// A current-thread runtime: the subscriber task cannot read while the test
// pushes, so the broadcast is guaranteed to lag.
#[tokio::test(flavor = "current_thread")]
async fn a_failed_work_unit_resync_after_missed_updates_is_shown_as_an_error() -> TestResult {
    // @step Given the TUI is subscribed to work-unit updates
    let mut harness = AppTestHarness::empty();
    harness.app.bootstrap().await?;
    harness.drain_pending().await?;
    // @step And listing work units fails with "daemon busy"
    harness.mock.fail_list_work_units("daemon busy");
    // @step When the work-unit updates arrive faster than the TUI reads them
    for _ in 0..OVERFLOWING_SNAPSHOTS {
        harness.mock.push_work_units(Vec::new());
    }
    harness
        .wait_for_self(
            |harness| harness.compositor_contains(DIALOG_ID),
            "the error dialog after the lagged resync",
        )
        .await?;
    // @step Then an error dialog reads "Work units could not be reloaded after missed updates: daemon busy"
    expect_error_dialog(
        &mut harness,
        "Work units could not be reloaded after missed updates: daemon busy",
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_work_unit_reload_after_a_daemon_switch_is_shown_as_an_error() -> TestResult {
    // @step Given daemon "beta" fails to list work units with "daemon busy"
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    let beta = Arc::new(MockBackend::new());
    beta.fail_list_work_units("daemon busy");
    // @step When the TUI switches to daemon "beta"
    switch_daemon(&mut harness, "beta", beta).await?;
    // @step Then an error dialog reads "Work units could not be reloaded after switching to daemon beta: daemon busy"
    expect_error_dialog(
        &mut harness,
        "Work units could not be reloaded after switching to daemon beta: daemon busy",
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_checkpoint_count_reload_after_a_daemon_switch_is_shown_as_an_error() -> TestResult
{
    // @step Given daemon "beta" fails to count checkpoints with "store locked"
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    let beta = Arc::new(MockBackend::new());
    beta.fail_checkpoint_counts("store locked");
    // @step When the TUI switches to daemon "beta"
    switch_daemon(&mut harness, "beta", beta).await?;
    // @step Then an error dialog reads "Checkpoint counts could not be reloaded after switching to daemon beta: store locked"
    expect_error_dialog(
        &mut harness,
        "Checkpoint counts could not be reloaded after switching to daemon beta: store locked",
    )
}
