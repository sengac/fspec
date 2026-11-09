use super::*;

/// If metadata is set BEFORE start_capture, session.start should be emitted
/// immediately with the real provider/model values.
#[test]
#[serial]
fn test_session_start_records_metadata_when_set_before_start_capture() {
    let temp_dir = setup_test_data_dir();
    let mut manager = DebugCaptureManager::new().unwrap();
    let dir = temp_dir.path().join("debug").join("session-before");
    manager.set_debug_directory_raw(dir);

    manager.set_session_metadata(SessionMetadata {
        provider: Some("claude-rhai".to_string()),
        model: Some("opus-4.7".to_string()),
        context_window: Some(200000),
        max_output_tokens: Some(16384),
    });

    let file = manager.start_capture().unwrap();

    // session.start should have been emitted by start_capture with real metadata
    let events = read_events(std::path::Path::new(&file));
    let start_event = events
        .iter()
        .find(|e| e["eventType"] == "session.start")
        .expect("session.start must be present");
    assert_eq!(start_event["data"]["provider"], "claude-rhai");
    assert_eq!(start_event["data"]["model"], "opus-4.7");
    assert_eq!(start_event["data"]["contextWindow"], 200000);

    manager.stop_capture().unwrap();

    // Exactly one session.start and one session.end, in that order.
    let events = read_events(std::path::Path::new(&file));
    let start_count = events
        .iter()
        .filter(|e| e["eventType"] == "session.start")
        .count();
    let end_count = events
        .iter()
        .filter(|e| e["eventType"] == "session.end")
        .count();
    assert_eq!(start_count, 1, "exactly one session.start event");
    assert_eq!(end_count, 1, "exactly one session.end event");
    assert_eq!(events[0]["eventType"], "session.start");
    assert_eq!(events[events.len() - 1]["eventType"], "session.end");
}

/// If metadata is set AFTER start_capture (the common NAPI sequence),
/// session.start emission is deferred and then flushed at metadata time
/// with the REAL provider/model — not "unknown".
#[test]
#[serial]
fn test_session_start_deferred_until_metadata_set_after_start_capture() {
    let temp_dir = setup_test_data_dir();
    let mut manager = DebugCaptureManager::new().unwrap();
    let dir = temp_dir.path().join("debug").join("session-after");
    manager.set_debug_directory_raw(dir);

    let file = manager.start_capture().unwrap();

    // File must exist but session.start is NOT yet written.
    let events = read_events(std::path::Path::new(&file));
    assert!(
        events.iter().all(|e| e["eventType"] != "session.start"),
        "session.start must be deferred until metadata is set"
    );

    manager.set_session_metadata(SessionMetadata {
        provider: Some("claude-rhai".to_string()),
        model: Some("opus-4.7".to_string()),
        context_window: Some(200000),
        max_output_tokens: None,
    });

    let events = read_events(std::path::Path::new(&file));
    let start_event = events
        .iter()
        .find(|e| e["eventType"] == "session.start")
        .expect("session.start must be emitted after metadata set");
    assert_eq!(
        start_event["data"]["provider"], "claude-rhai",
        "provider must reflect real value, not 'unknown'"
    );
    assert_eq!(
        start_event["data"]["model"], "opus-4.7",
        "model must reflect real value, not 'unknown'"
    );

    manager.stop_capture().unwrap();
}

/// If metadata is NEVER set (user toggles /debug then quits immediately),
/// stop_capture must still emit a fallback session.start so the file
/// contains the expected start/end pair. Values are the "unknown" defaults.
#[test]
#[serial]
fn test_session_start_fallback_emitted_on_stop_when_no_metadata() {
    let temp_dir = setup_test_data_dir();
    let mut manager = DebugCaptureManager::new().unwrap();
    let dir = temp_dir.path().join("debug").join("session-fallback");
    manager.set_debug_directory_raw(dir);

    let file = manager.start_capture().unwrap();
    manager.stop_capture().unwrap();

    let events = read_events(std::path::Path::new(&file));
    assert_eq!(events.len(), 2, "exactly session.start + session.end");
    assert_eq!(events[0]["eventType"], "session.start");
    assert_eq!(events[1]["eventType"], "session.end");
    // Fallback values - we can't do better without metadata, but the event exists.
    assert_eq!(events[0]["data"]["provider"], "unknown");
    assert_eq!(events[0]["data"]["model"], "unknown");
}

/// Metadata that arrives after stop_capture must not re-emit session.start.
#[test]
#[serial]
fn test_set_session_metadata_after_stop_does_not_emit() {
    let temp_dir = setup_test_data_dir();
    let mut manager = DebugCaptureManager::new().unwrap();
    let dir = temp_dir.path().join("debug").join("session-late");
    manager.set_debug_directory_raw(dir);

    let file = manager.start_capture().unwrap();
    manager.stop_capture().unwrap();

    // Late metadata set on a disabled manager must not append anything.
    manager.set_session_metadata(SessionMetadata {
        provider: Some("claude-rhai".to_string()),
        model: Some("opus-4.7".to_string()),
        context_window: Some(200000),
        max_output_tokens: None,
    });

    let events = read_events(std::path::Path::new(&file));
    let start_count = events
        .iter()
        .filter(|e| e["eventType"] == "session.start")
        .count();
    assert_eq!(start_count, 1, "no duplicate session.start events");
}
