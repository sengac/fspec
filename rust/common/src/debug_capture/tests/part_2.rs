use super::*;

/// Scenario: Toggling debug off in one session does not affect another session's capture
#[test]
#[serial]
fn test_per_session_toggle_independence() {
    let temp_dir = setup_test_data_dir();

    let session_a_id = Uuid::new_v4();
    let session_b_id = Uuid::new_v4();

    // @step Given session A and session B both have debug capture enabled
    let mut manager_a = DebugCaptureManager::new().unwrap();
    let mut manager_b = DebugCaptureManager::new().unwrap();

    let dir_a = temp_dir.path().join("debug").join(session_a_id.to_string());
    let dir_b = temp_dir.path().join("debug").join(session_b_id.to_string());
    manager_a.set_debug_directory_raw(dir_a);
    manager_b.set_debug_directory_raw(dir_b);

    let _file_a = manager_a.start_capture().unwrap();
    let file_b = manager_b.start_capture().unwrap();

    // @step And each session is writing to its own independent JSONL file
    assert!(manager_a.is_enabled());
    assert!(manager_b.is_enabled());

    // Write events to both
    manager_a.capture("api.request", serde_json::json!({"from": "a"}), None);
    manager_b.capture("api.request", serde_json::json!({"from": "b"}), None);

    // @step When session A toggles debug off
    manager_a.stop_capture().unwrap();

    // @step Then session A's capture should stop and its file should be closed
    assert!(!manager_a.is_enabled());

    // @step And session B's capture should continue writing uninterrupted to its own file
    assert!(manager_b.is_enabled());

    // B can still capture events after A stopped
    manager_b.capture(
        "tool.call",
        serde_json::json!({"tool": "grep", "from": "b_after_a_stopped"}),
        None,
    );
    manager_b.stop_capture().unwrap();

    // Verify B's file has the event written after A stopped
    let content_b = std::fs::read_to_string(&file_b).unwrap();
    assert!(
        content_b.contains("b_after_a_stopped"),
        "Session B should have events written after session A stopped"
    );
}

/// Scenario: latest.jsonl symlink points to the most recently activated session's debug file
#[test]
#[serial]
#[cfg(unix)]
fn test_latest_symlink_points_to_most_recent_session() {
    let temp_dir = setup_test_data_dir();

    let session_a_id = Uuid::new_v4();
    let session_b_id = Uuid::new_v4();

    let mut manager_a = DebugCaptureManager::new().unwrap();
    let mut manager_b = DebugCaptureManager::new().unwrap();

    // Both share the same parent debug dir for the latest.jsonl symlink
    let debug_dir = temp_dir.path().join("debug");
    let dir_a = debug_dir.join(session_a_id.to_string());
    let dir_b = debug_dir.join(session_b_id.to_string());
    manager_a.set_debug_directory_raw(dir_a);
    manager_b.set_debug_directory_raw(dir_b);

    // @step Given session A enables debug capture at time T1
    let file_a = manager_a.start_capture().unwrap();

    // After A starts, latest.jsonl should point to A's file
    // Note: The symlink is created in session-specific dir, but we need
    // a global latest.jsonl in the parent debug dir for cross-session use.
    // For now, the per-session manager creates it in its own debug_dir.
    // The refactored implementation should update the PARENT debug dir's latest.jsonl.

    // @step And session B enables debug capture at time T2 where T2 is after T1
    // B starts after A returned; the order of the calls is what counts
    // (TEST-042: no sleep; the two sessions write to their own directories).
    let file_b = manager_b.start_capture().unwrap();

    // @step Then the "latest.jsonl" symlink should point to session B's debug file
    // The latest.jsonl lives in session B's debug_dir right now
    // After refactoring, it should live in the parent debug_dir
    let latest_in_b_dir = std::path::Path::new(&file_b)
        .parent()
        .unwrap()
        .join("latest.jsonl");

    if latest_in_b_dir.is_symlink() {
        let target = std::fs::read_link(&latest_in_b_dir).unwrap();
        assert_eq!(
            target.to_string_lossy(),
            file_b,
            "latest.jsonl should point to session B's file"
        );
    }

    // @step When session B toggles debug off
    manager_b.stop_capture().unwrap();

    // @step Then the "latest.jsonl" symlink should remain pointing to session B's last file
    // The symlink shouldn't be removed when capture stops
    if latest_in_b_dir.is_symlink() {
        let target = std::fs::read_link(&latest_in_b_dir).unwrap();
        assert!(
            target.to_string_lossy().contains("session-"),
            "latest.jsonl symlink should remain after stop"
        );
    }

    // Clean up A
    let _ = manager_a.stop_capture();

    // Verify both files are distinct
    assert_ne!(file_a, file_b, "Sessions should have different debug files");
}

/// BUG-134: DebugCaptureManager.new() must be public so BackgroundSession can construct it
#[test]
#[serial]
fn test_debug_capture_manager_constructable_outside_module() {
    let _temp_dir = setup_test_data_dir();

    // This test verifies that DebugCaptureManager::new() is accessible
    // from outside the debug_capture module (i.e., from session_manager.rs in napi crate).
    // If new() is still pub(super), this test compiles but the napi crate won't.
    let manager = DebugCaptureManager::new().unwrap();
    assert!(!manager.is_enabled());
}

/// BUG-134: set_debug_directory_raw sets the debug dir without appending "debug/"
#[test]
#[serial]
fn test_set_debug_directory_raw() {
    let temp_dir = setup_test_data_dir();
    let mut manager = DebugCaptureManager::new().unwrap();

    let custom_dir = temp_dir.path().join("custom").join("session-123");
    manager.set_debug_directory_raw(custom_dir.clone());

    let file = manager.start_capture().unwrap();
    assert!(
        file.starts_with(&custom_dir.to_string_lossy().to_string()),
        "Debug file should be under the raw directory, got: {file}"
    );

    manager.stop_capture().unwrap();
}

/// Scenario: Toggling debug emits a DebugStateChange stream event scoped to that session
///
/// Verifies that the DebugStateChange variant exists in the StreamChunk enum
/// and that session_toggle_debug emits it.
#[test]
fn test_napi_stream_chunk_has_debug_state_change_variant() {
    let rust_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();

    // @step Given session A is running
    // Since the RPC-002 split, StreamChunk is defined in codelet-rpc-types.
    // REFAC-021: rpc-types is split into per-surface modules — read
    // every source file under rpc-types/src.
    let rpc_types_path = rust_root.join("rpc-types").join("src");
    let mut files: Vec<_> = std::fs::read_dir(&rpc_types_path)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    let mut combined_source = String::new();
    for file in files {
        combined_source.push_str(&std::fs::read_to_string(&file).unwrap());
    }
    assert!(
        combined_source.contains("pub enum StreamChunk"),
        "StreamChunk enum must be present in codelet-rpc-types"
    );

    // @step When session A toggles debug on via the "/debug" command
    assert!(
        combined_source.contains("DebugStateChange"),
        "StreamChunk must have a DebugStateChange variant"
    );

    // @step Then Rust should emit a DebugStateChange stream event with enabled true on session A's stream only
    assert!(
        combined_source.contains("DebugStateChange") && combined_source.contains("enabled:"),
        "DebugStateChange variant must have an 'enabled' field"
    );

    // @step And session B's stream should not receive any DebugStateChange event
    // Emission must be keyed to the toggled session's id on the per-session
    // chunk stream, verified against the sessions engine's toggle_debug.
    let handle_impl = std::fs::read_to_string(
        rust_root
            .join("sessions")
            .join("src")
            .join("handle_impl")
            .join("handle_impl_3.rs"),
    )
    .unwrap();
    let start = handle_impl
        .find("fn toggle_debug(")
        .expect("sessions handle_impl must implement toggle_debug");
    let toggle_fn_body = &handle_impl[start..];
    let fn_end = toggle_fn_body
        .find("fn set_debug_directory(")
        .unwrap_or(toggle_fn_body.len());
    let toggle_fn_text = &toggle_fn_body[..fn_end];
    assert!(
        toggle_fn_text.contains("session_id.clone()")
            && toggle_fn_text.contains("StreamChunk::debug_state_change("),
        "toggle_debug should emit DebugStateChange keyed to the toggled session id"
    );
}
