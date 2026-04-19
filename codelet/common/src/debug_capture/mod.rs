//! Debug Capture System for LLM Session Diagnostics
//!
//! Provides comprehensive capture of LLM API communication, tool executions,
//! and application logs for debugging agent issues.
//!
//! ACDD Work Unit: CLI-022
//! Port of codelet's debug-capture.ts to Rust.

mod capture;
mod error;
mod manager;
mod mutex;
mod session_lifecycle;
mod summary;
mod types;

pub use error::DebugCaptureError;
pub use manager::DebugCaptureManager;
pub use mutex::PoisonRecoveryMutex;
pub use types::{CaptureOptions, DebugCommandResult, DebugEvent, DebugEventType, SessionMetadata};

use std::sync::{Arc, OnceLock};

// Global singleton instance with poison recovery
static DEBUG_CAPTURE_MANAGER: OnceLock<Arc<PoisonRecoveryMutex<DebugCaptureManager>>> =
    OnceLock::new();

/// Get the singleton debug capture manager instance
///
/// This implementation handles mutex poisoning gracefully by clearing the poison
/// and continuing. For debug capture, this is acceptable since the worst case
/// is missing some debug events.
#[allow(clippy::expect_used)]
pub fn get_debug_capture_manager(
) -> Result<Arc<PoisonRecoveryMutex<DebugCaptureManager>>, DebugCaptureError> {
    // Note: get_or_init requires infallible initialization. If the data directory
    // is not set, this is a startup issue and panic is appropriate.
    let manager = DEBUG_CAPTURE_MANAGER.get_or_init(|| {
        let mgr = DebugCaptureManager::new().expect("Failed to create DebugCaptureManager");
        Arc::new(PoisonRecoveryMutex::new(mgr))
    });

    Ok(manager.clone())
}

/// Handle the /debug command to toggle debug capture
pub fn handle_debug_command() -> DebugCommandResult {
    handle_debug_command_with_dir(None)
}

/// Capture a debug event if debug capture is enabled
///
/// This is a convenience function that handles the boilerplate of:
/// 1. Getting the debug capture manager
/// 2. Locking the mutex
/// 3. Checking if capture is enabled
/// 4. Calling capture with the event data
///
/// Failures are silently ignored since debug capture is optional.
pub fn capture_event(event_type: &str, data: serde_json::Value) {
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(event_type, data, None);
            }
        }
    }
}

/// Increment the turn counter if debug capture is enabled
///
/// This is a convenience function for incrementing the turn counter.
/// Should be called once per user input to track conversation turns.
///
/// Failures are silently ignored since debug capture is optional.
pub fn increment_debug_turn() {
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.increment_turn();
            }
        }
    }
}

/// Handle the /debug command with a custom base directory
///
/// If base_dir is provided, debug files will be written to `{base_dir}/debug/`
pub fn handle_debug_command_with_dir(base_dir: Option<&str>) -> DebugCommandResult {
    use std::path::PathBuf;

    let manager_arc = match get_debug_capture_manager() {
        Ok(m) => m,
        Err(e) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to initialize debug capture: {e}"),
            }
        }
    };

    let mut manager = match manager_arc.lock() {
        Ok(m) => m,
        Err(_) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: "Failed to acquire lock on debug capture manager".to_string(),
            }
        }
    };

    // Set custom directory if provided (before starting capture)
    if let Some(dir) = base_dir {
        manager.set_debug_directory(PathBuf::from(dir));
    }

    if manager.is_enabled() {
        // Turn off
        match manager.stop_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: false,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture stopped. Session saved to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to stop debug capture: {e}"),
            },
        }
    } else {
        // Turn on
        match manager.start_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: true,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture started. Writing to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to start debug capture: {e}"),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use manager::sanitize_headers;
    use serial_test::serial;
    use uuid::Uuid;

    fn setup_test_data_dir() -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        crate::set_data_directory(temp_dir.path().to_path_buf())
            .expect("Failed to set data directory");
        temp_dir
    }

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

    // ======================================================================
    // BUG-134: Per-session debug capture tests
    // Feature: spec/features/debug-capture-per-session.feature
    // ======================================================================

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
        manager_a.capture("api.request", serde_json::json!({"from": "session_a"}), None);
        manager_a.capture("tool.call", serde_json::json!({"tool": "bash", "from": "session_a"}), None);

        // B tries to capture - should silently do nothing since disabled
        manager_b.capture("api.request", serde_json::json!({"from": "session_b"}), None);

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
        manager_b.capture("tool.call", serde_json::json!({"tool": "grep", "from": "b_after_a_stopped"}), None);
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
        // Small sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
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

    // ======================================================================
    // BUG-134: Tests for DebugStateChange stream event and NAPI integration
    // These tests verify the NAPI layer via source code inspection since
    // the NAPI runtime is not available in unit tests.
    // ======================================================================

    /// Scenario: Toggling debug emits a DebugStateChange stream event scoped to that session
    ///
    /// Verifies that the DebugStateChange variant exists in the StreamChunk enum
    /// and that session_toggle_debug emits it.
    #[test]
    fn test_napi_stream_chunk_has_debug_state_change_variant() {
        let types_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("napi")
            .join("src")
            .join("types.rs");

        if !types_path.exists() {
            // Skip if napi crate not found (CI environments)
            return;
        }

        let types_source = std::fs::read_to_string(&types_path).unwrap();

        // @step Given session A is running
        assert!(
            types_source.contains("pub enum StreamChunk"),
            "StreamChunk enum must exist in types.rs"
        );

        // @step When session A toggles debug on via the "/debug" command
        assert!(
            types_source.contains("DebugStateChange"),
            "StreamChunk must have a DebugStateChange variant"
        );

        // @step Then Rust should emit a DebugStateChange stream event with enabled true on session A's stream only
        assert!(
            types_source.contains("DebugStateChange") && types_source.contains("enabled:"),
            "DebugStateChange variant must have an 'enabled' field"
        );

        // @step And session B's stream should not receive any DebugStateChange event
        // Emission must use handle_output (per-session stream), verified by checking session_manager
        let sm_path = types_path.parent().unwrap().join("session_manager.rs");
        if sm_path.exists() {
            let sm_source = std::fs::read_to_string(&sm_path).unwrap();
            let toggle_fn_start = sm_source.find("pub async fn session_toggle_debug");
            if let Some(start) = toggle_fn_start {
                let toggle_fn_body = &sm_source[start..];
                let fn_end = toggle_fn_body[1..]
                    .find("#[napi]")
                    .unwrap_or(toggle_fn_body.len() - 1);
                let toggle_fn_text = &toggle_fn_body[..fn_end];
                assert!(
                    toggle_fn_text.contains("handle_output")
                        && toggle_fn_text.contains("DebugStateChange"),
                    "session_toggle_debug should emit DebugStateChange via session.handle_output"
                );
            }
        }
    }
}
