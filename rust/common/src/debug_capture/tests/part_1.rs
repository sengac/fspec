use super::*;

#[test]
fn test_sanitize_headers_redacts_sensitive() {
    let headers = serde_json::json!({
        "authorization": "Bearer secret",
        "x-api-key": "key123",
        "content-type": "application/json"
    });

    let sanitized = sanitize_headers(&headers);

    assert_eq!(
        sanitized.get("authorization").unwrap().as_str().unwrap(),
        "[REDACTED]"
    );
    assert_eq!(
        sanitized.get("x-api-key").unwrap().as_str().unwrap(),
        "[REDACTED]"
    );
    assert_eq!(
        sanitized.get("content-type").unwrap().as_str().unwrap(),
        "application/json"
    );
}

#[test]
#[serial]
fn test_format_duration() {
    let _temp_dir = setup_test_data_dir();
    let manager = DebugCaptureManager::new().unwrap();

    assert_eq!(manager.format_duration(500), "0s");
    assert_eq!(manager.format_duration(5000), "5s");
    assert_eq!(manager.format_duration(65000), "1m 5s");
    assert_eq!(manager.format_duration(3665000), "1h 1m 5s");
}

/// Feature: Refactor DebugCaptureManager to be truly per-session
///
/// Scenario: DebugCaptureManager is owned by BackgroundSession not a global singleton
///
/// Verifies that multiple independent DebugCaptureManager instances can be
/// created (one per session), each with its own state.
#[test]
#[serial]
fn test_per_session_manager_independent_instances() {
    // @step Given the process has multiple BackgroundSession instances
    let _temp_dir = setup_test_data_dir();

    let manager_a = DebugCaptureManager::new().unwrap();
    let manager_b = DebugCaptureManager::new().unwrap();

    // @step Then each BackgroundSession should own its own DebugCaptureManager instance
    // Both managers exist independently - they are NOT the same instance
    assert!(!manager_a.is_enabled());
    assert!(!manager_b.is_enabled());

    // @step And there should be no process-wide OnceLock singleton for debug capture
    // Each manager is independently created via ::new(), not from a global singleton.
    // The BackgroundSession struct owns its own Arc<PoisonRecoveryMutex<DebugCaptureManager>>.

    // @step And each manager should be independently toggleable
    // Wrap in PoisonRecoveryMutex like BackgroundSession will
    let mgr_a = Arc::new(PoisonRecoveryMutex::new(manager_a));
    let mgr_b = Arc::new(PoisonRecoveryMutex::new(manager_b));

    // Enable A only
    {
        let mut a = mgr_a.lock().unwrap();
        a.start_capture().unwrap();
    }

    // A is enabled, B is not
    assert!(mgr_a.lock().unwrap().is_enabled());
    assert!(!mgr_b.lock().unwrap().is_enabled());

    // Enable B
    {
        let mut b = mgr_b.lock().unwrap();
        b.start_capture().unwrap();
    }

    // Both enabled independently
    assert!(mgr_a.lock().unwrap().is_enabled());
    assert!(mgr_b.lock().unwrap().is_enabled());

    // Disable A - B should remain enabled
    {
        let mut a = mgr_a.lock().unwrap();
        a.stop_capture().unwrap();
    }

    assert!(!mgr_a.lock().unwrap().is_enabled());
    assert!(mgr_b.lock().unwrap().is_enabled());

    // Clean up B
    mgr_b.lock().unwrap().stop_capture().unwrap();
}

/// Scenario: Each session writes to its own session-specific debug file path
///
/// Verifies that when a session id is provided, debug files are written
/// under a session-specific directory: ~/.fspec/debug/{session_id}/
#[test]
#[serial]
fn test_per_session_debug_file_path_includes_session_id() {
    let temp_dir = setup_test_data_dir();
    let session_id = Uuid::parse_str("aaaaaaaa-1111-2222-3333-444444444444").unwrap();

    // @step Given session A has id "aaaa-1111"
    let mut manager = DebugCaptureManager::new().unwrap();

    // @step When session A enables debug capture
    // Set the debug directory to include the session id
    let session_debug_dir = temp_dir.path().join("debug").join(session_id.to_string());
    manager.set_debug_directory_raw(session_debug_dir);
    let session_file = manager.start_capture().unwrap();

    // @step Then a debug JSONL file should be created under "~/.fspec/debug/aaaa-1111/"
    assert!(
        session_file.contains(&session_id.to_string()),
        "Debug file path should contain session id, got: {session_file}"
    );

    // @step And the filename should follow the pattern "session-{timestamp}.jsonl"
    assert!(
        session_file.contains("session-") && session_file.ends_with(".jsonl"),
        "Debug file should match session-{{timestamp}}.jsonl pattern, got: {session_file}"
    );

    // Verify the file actually exists
    assert!(
        std::path::Path::new(&session_file).exists(),
        "Debug file should exist on disk"
    );

    manager.stop_capture().unwrap();
}

/// Scenario: Debug capture for one session does not leak into another session's log file
///
/// Two independent managers capture events - each writes ONLY to its own file.
#[test]
#[serial]
fn test_per_session_capture_no_leak_between_sessions() {
    let temp_dir = setup_test_data_dir();

    let session_a_id = Uuid::new_v4();
    let session_b_id = Uuid::new_v4();

    // @step Given session A and session B are running concurrently
    let mut manager_a = DebugCaptureManager::new().unwrap();
    let mut manager_b = DebugCaptureManager::new().unwrap();

    // Set per-session directories
    let dir_a = temp_dir.path().join("debug").join(session_a_id.to_string());
    let dir_b = temp_dir.path().join("debug").join(session_b_id.to_string());
    manager_a.set_debug_directory_raw(dir_a);
    manager_b.set_debug_directory_raw(dir_b.clone());

    // @step And session A has debug capture enabled
    let file_a = manager_a.start_capture().unwrap();

    // @step And session B has debug capture disabled
    // (B not started - disabled by default)
    assert!(!manager_b.is_enabled());

    // @step When session B makes API calls and processes tool results
    // Capture some events on A
    manager_a.capture(
        "api.request",
        serde_json::json!({"from": "session_a"}),
        None,
    );
    manager_a.capture(
        "tool.call",
        serde_json::json!({"tool": "bash", "from": "session_a"}),
        None,
    );

    // B tries to capture - should silently do nothing since disabled
    manager_b.capture(
        "api.request",
        serde_json::json!({"from": "session_b"}),
        None,
    );

    manager_a.stop_capture().unwrap();

    // @step Then session A's JSONL file should contain only session A's events
    let content_a = std::fs::read_to_string(&file_a).unwrap();
    assert!(
        content_a.contains("session_a"),
        "Session A file should contain session A events"
    );
    assert!(
        !content_a.contains("session_b"),
        "Session A file should NOT contain session B events"
    );

    // @step And session B should have no debug JSONL file
    // B's directory should not even exist (or be empty)
    let b_has_files = dir_b.exists()
        && std::fs::read_dir(&dir_b)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
    assert!(
        !b_has_files,
        "Session B should have no debug files since capture was never enabled"
    );
}
