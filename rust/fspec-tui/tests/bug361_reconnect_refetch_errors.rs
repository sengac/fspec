//! BUG-361 — a failed work-unit refetch after reconnect is shown as an
//! error instead of leaving a silently stale board.
//!
//! Feature: spec/features/reconnect-work-units-refetch-errors.feature

use codelet_fspec_tui::Action;
use codelet_test_helpers::TestResult;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::harness::AppTestHarness;

const DIALOG_ID: &str = "session-storage-dialog";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_work_unit_refetch_after_reconnect_is_shown_as_an_error() -> TestResult {
    // @step Given the TUI is connected to a daemon
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    // @step And listing work units fails with "daemon busy"
    harness.mock.fail_list_work_units("daemon busy");
    // @step When the TUI reconnects
    harness.app.dispatch(Action::Reconnected);
    harness.drain_pending().await?;
    // @step Then an error dialog reads "Work units could not be reloaded after reconnect: daemon busy"
    if !harness.compositor_contains(DIALOG_ID) {
        return Err("no error dialog is shown after the failed refetch".into());
    }
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    let screen: String = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let expected = "Work units could not be reloaded after reconnect: daemon busy";
    if !screen.contains(expected) {
        return Err(format!("{expected:?} is not shown in:\n{screen}").into());
    }
    Ok(())
}
