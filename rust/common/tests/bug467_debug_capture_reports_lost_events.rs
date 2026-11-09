//! BUG-467 — debug capture reports events and summaries it cannot write.
//!
//! Feature: spec/features/debug-capture-reports-lost-events.feature

use codelet_common::debug_capture::{DebugCaptureError, DebugCaptureManager};
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn manager() -> Result<(tempfile::TempDir, DebugCaptureManager), Box<dyn std::error::Error>> {
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let mut manager = DebugCaptureManager::new()?;
    manager.set_debug_directory_raw(data.path().join("debug"));
    Ok((data, manager))
}

#[test]
#[serial]
fn stopping_a_capture_that_lost_events_reports_them() -> TestResult {
    // @step Given a started debug capture whose session file is removed and replaced by a directory
    let (_data, mut manager) = manager()?;
    let session = manager.start_capture()?;
    std::fs::remove_file(&session)?;
    std::fs::create_dir(&session)?;
    // @step When an event is captured and capture stops
    manager.capture("api.request", serde_json::json!({}), None);
    let stopped = manager.stop_capture();
    // @step Then stopping fails reporting 1 lost event and the write error
    match stopped {
        Err(DebugCaptureError::EventsLost { count, .. }) => assert!(count >= 1, "{count}"),
        other => return Err(format!("expected EventsLost, got {other:?}").into()),
    }
    Ok(())
}

#[test]
#[serial]
fn stopping_a_capture_that_is_off_is_an_error() -> TestResult {
    // @step Given a debug capture manager that has not started capture
    let (_data, mut manager) = manager()?;
    // @step When capture stops
    let stopped = manager.stop_capture();
    // @step Then stopping fails saying capture is not running
    assert!(
        matches!(stopped, Err(DebugCaptureError::NotCapturing)),
        "{stopped:?}"
    );
    Ok(())
}
