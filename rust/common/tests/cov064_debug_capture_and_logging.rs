//! COV-064 — debug capture and logging.
//!
//! Feature: spec/features/debug-capture-and-logging.feature

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use codelet_common::debug_capture::{CaptureOptions, DebugCaptureManager, SessionMetadata};
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A manager whose debug directory is `<temp>/debug` (not created yet).
fn manager() -> Result<(tempfile::TempDir, DebugCaptureManager), Box<dyn std::error::Error>> {
    let data = tempfile::tempdir()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let mut manager = DebugCaptureManager::new()?;
    manager.set_debug_directory_raw(data.path().join("debug"));
    Ok((data, manager))
}

fn events(path: &str) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    std::fs::read_to_string(path)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

#[test]
#[serial]
fn starting_a_capture_creates_a_private_debug_directory_and_a_session_file() -> TestResult {
    // @step Given a debug capture manager whose debug directory does not exist yet
    let (data, mut manager) = manager()?;
    let directory = data.path().join("debug");
    assert!(!directory.exists());
    // @step When capture starts
    let session = manager.start_capture()?;
    // @step Then the debug directory exists with 0700 permissions
    let mode = std::fs::metadata(&directory)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    // @step And an empty session file named session-<timestamp>.jsonl exists in it
    let path = std::path::Path::new(&session);
    assert_eq!(path.parent(), Some(directory.as_path()));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("no file name")?;
    assert!(
        name.starts_with("session-") && name.ends_with(".jsonl"),
        "{name}"
    );
    assert_eq!(std::fs::read_to_string(path)?, "");
    Ok(())
}

#[test]
#[serial]
fn captured_events_are_numbered_json_lines_with_redacted_sensitive_headers() -> TestResult {
    // @step Given a started debug capture in turn 2
    let (_data, mut manager) = manager()?;
    let session = manager.start_capture()?;
    manager.increment_turn();
    manager.increment_turn();
    // @step When an "api.request" event with an Authorization header and request id "req-1" is captured
    manager.capture(
        "api.request",
        serde_json::json!({"headers": {"Authorization": "Bearer secret", "accept": "json"}}),
        Some(CaptureOptions {
            request_id: Some("req-1".to_string()),
        }),
    );
    // @step Then the session file holds one JSON line with sequence 0, turnId 2 and requestId "req-1"
    let written = events(&session)?;
    assert_eq!(written.len(), 1, "{written:?}");
    assert_eq!(written[0]["sequence"], 0);
    assert_eq!(written[0]["turnId"], 2);
    assert_eq!(written[0]["requestId"], "req-1");
    assert_eq!(written[0]["eventType"], "api.request");
    // @step And the Authorization header reads "[REDACTED]"
    assert_eq!(written[0]["data"]["headers"]["Authorization"], "[REDACTED]");
    assert_eq!(written[0]["data"]["headers"]["accept"], "json");
    Ok(())
}

#[test]
#[serial]
fn nothing_is_captured_while_capture_is_disabled() -> TestResult {
    // @step Given a debug capture manager that has not started capture
    let (data, mut manager) = manager()?;
    // @step When an event is captured
    manager.capture("api.request", serde_json::json!({}), None);
    // @step Then no session file is written
    assert!(!manager.is_enabled());
    assert!(!data.path().join("debug").exists());
    Ok(())
}

#[test]
#[serial]
fn stopping_a_capture_records_the_session_start_and_end_and_writes_a_summary() -> TestResult {
    // @step Given a started debug capture with provider "anthropic" and model "claude-sonnet-4"
    let (_data, mut manager) = manager()?;
    manager.start_capture()?;
    manager.set_session_metadata(SessionMetadata {
        provider: Some("anthropic".to_string()),
        model: Some("claude-sonnet-4".to_string()),
        context_window: None,
        max_output_tokens: None,
    });
    // @step When capture stops
    let session = manager.stop_capture()?;
    // @step Then the session file holds exactly one session.start naming the provider and model and one session.end after it
    let written = events(&session)?;
    let kinds: Vec<&str> = written
        .iter()
        .filter_map(|event| event["eventType"].as_str())
        .collect();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "session.start")
            .count(),
        1,
        "{kinds:?}"
    );
    assert_eq!(kinds.last(), Some(&"session.end"), "{kinds:?}");
    let start = written
        .iter()
        .find(|event| event["eventType"] == "session.start")
        .ok_or("no session.start")?;
    assert_eq!(start["data"]["provider"], "anthropic");
    assert_eq!(start["data"]["model"], "claude-sonnet-4");
    // @step And a .summary.md file is written next to the session file
    let summary = session.replace(".jsonl", ".summary.md");
    assert!(std::fs::read_to_string(&summary)?.contains("# Debug Session Summary"));
    Ok(())
}

#[test]
fn rust_logs_go_to_private_daily_rotated_files() -> TestResult {
    // @step Given an empty logs directory
    let base = tempfile::tempdir()?;
    let directory = base.path().join("logs/private");
    // @step When the private log appender is built and a line is written
    let mut appender = codelet_common::logging::private_files::appender(&directory, "rust")?;
    writeln!(appender, "a log line")?;
    appender.flush()?;
    // @step Then the directory has 0700 permissions
    let mode = std::fs::metadata(&directory)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
    // @step And the line is in a file whose name starts with "rust"
    let files: Vec<std::path::PathBuf> = std::fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0]
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("no name")?;
    assert!(name.starts_with("rust"), "{name}");
    assert_eq!(std::fs::read_to_string(&files[0])?, "a log line\n");
    Ok(())
}
