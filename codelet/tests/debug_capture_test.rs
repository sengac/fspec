
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/debug-capture-system-for-llm-session-diagnostics.feature
//!
//! Tests for the debug capture system that records LLM session diagnostics.
//! ACDD Work Unit: CLI-022
//!
//! This is a one-for-one port of codelet's debug-capture.test.ts to Rust.
//!
//! NOTE: These tests must run serially because they share a global singleton
//! (DebugCaptureManager) that maintains state across tests.

use std::fs;
use std::path::PathBuf;

use serial_test::serial;

// These imports will fail until implementation exists (red phase)
use codelet::debug_capture::{get_debug_capture_manager, handle_debug_command, DebugEvent};

fn get_test_debug_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".codelet")
        .join("debug")
}

fn cleanup_manager() {
    let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
    let mut manager = manager_arc.lock().expect("Failed to lock manager");
    if manager.is_enabled() {
        let _ = manager.stop_capture();
    }
}

// ============================================================================
// Scenario: Enable debug capture with /debug command
// ============================================================================
#[test]
#[serial]
fn test_enable_debug_capture_with_debug_command() {
    cleanup_manager();
    let test_debug_dir = get_test_debug_dir();

    // @step Given the agent is running
    let manager_arc = get_debug_capture_manager();
    assert!(manager_arc.is_ok(), "Manager should be defined");
    let manager_arc = manager_arc.expect("Failed to get manager");
    let manager = manager_arc.lock().expect("Failed to lock manager");

    // @step And debug capture is disabled
    assert!(
        !manager.is_enabled(),
        "Debug capture should be disabled initially"
    );
    drop(manager); // Release lock before handle_debug_command

    // @step When I enter the "/debug" command
    let result = handle_debug_command();

    // @step Then I should see "Debug capture started"
    assert!(
        result.message.contains("Debug capture started"),
        "Message should contain 'Debug capture started'"
    );

    // @step And I should see the path to the debug session file
    assert!(
        result.session_file.is_some(),
        "Session file should be defined"
    );
    assert!(
        result
            .session_file
            .as_ref()
            .unwrap()
            .contains(".codelet/debug/session-"),
        "Session file path should contain .codelet/debug/session-"
    );

    // @step And the debug directory "~/.codelet/debug/" should exist
    assert!(test_debug_dir.exists(), "Debug directory should exist");

    // @step And a new JSONL session file should be created
    let session_file = result.session_file.as_ref().unwrap();
    assert!(
        PathBuf::from(session_file).exists(),
        "Session file should exist"
    );
    assert!(
        session_file.ends_with(".jsonl"),
        "Session file should have .jsonl extension"
    );

    cleanup_manager();
}

// ============================================================================
// Scenario: Disable debug capture with /debug command
// ============================================================================
#[test]
#[serial]
fn test_disable_debug_capture_with_debug_command() {
    cleanup_manager();

    // @step Given the agent is running
    // @step And debug capture is enabled with an active session
    let start_result = handle_debug_command();
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let manager = manager_arc.lock().expect("Failed to lock manager");
        assert!(manager.is_enabled(), "Debug capture should be enabled");
    }
    let session_file = start_result.session_file.clone().unwrap();

    // @step When I enter the "/debug" command
    let stop_result = handle_debug_command();

    // @step Then I should see "Debug capture stopped"
    assert!(
        stop_result.message.contains("Debug capture stopped"),
        "Message should contain 'Debug capture stopped'"
    );

    // @step And I should see the path to the saved session file
    assert!(
        stop_result.session_file.is_some(),
        "Session file should be defined"
    );
    assert_eq!(
        stop_result.session_file.as_ref().unwrap(),
        &session_file,
        "Session file path should match"
    );

    // @step And the session file should be closed and complete
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let manager = manager_arc.lock().expect("Failed to lock manager");
        assert!(!manager.is_enabled(), "Debug capture should be disabled");
    }
    let content = fs::read_to_string(&session_file).expect("Should read session file");
    assert!(
        content.contains("session.start"),
        "Session file should contain session.start"
    );
    assert!(
        content.contains("session.end"),
        "Session file should contain session.end"
    );
}

// ============================================================================
// Scenario: Capture API request with correlation ID
// ============================================================================
#[test]
#[serial]
fn test_capture_api_request_with_correlation_id() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let manager = manager_arc.lock().expect("Failed to lock manager");
        assert!(manager.is_enabled(), "Debug capture should be enabled");
    }

    // @step When the agent makes an LLM API request
    let request_id = "req-test-123";
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "api.request",
            serde_json::json!({
                "requestId": request_id,
                "provider": "claude",
                "model": "claude-sonnet-4",
                "headers": { "authorization": "[REDACTED]" },
                "body": { "messages": [{ "role": "user", "content": "test" }] }
            }),
            None,
        );
    }

    // @step Then an "api.request" event should be written to the debug stream
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let api_request_event = events.iter().find(|e| e.event_type == "api.request");
    assert!(
        api_request_event.is_some(),
        "api.request event should exist"
    );
    let api_request_event = api_request_event.unwrap();

    // @step And the event should contain a unique "requestId"
    assert_eq!(
        api_request_event.data["requestId"].as_str().unwrap(),
        request_id,
        "requestId should match"
    );

    // @step And the event should contain the request headers with credentials redacted
    assert_eq!(
        api_request_event.data["headers"]["authorization"]
            .as_str()
            .unwrap(),
        "[REDACTED]",
        "authorization header should be redacted"
    );

    // @step And the event should contain the full request payload
    assert!(
        api_request_event.data["body"].is_object(),
        "body should be defined"
    );
    assert!(
        api_request_event.data["body"]["messages"].is_array(),
        "body.messages should be defined"
    );

    // @step And the event should contain a timestamp
    assert!(
        !api_request_event.timestamp.is_empty(),
        "timestamp should be defined"
    );
}

// ============================================================================
// Scenario: Capture API response with correlation ID
// ============================================================================
#[test]
#[serial]
fn test_capture_api_response_with_correlation_id() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step And an API request was made with requestId "req-123"
    let request_id = "req-123";
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "api.request",
            serde_json::json!({
                "requestId": request_id,
                "provider": "claude",
                "model": "claude-sonnet-4",
                "headers": {},
                "body": {}
            }),
            None,
        );

        // @step When the LLM API returns a response
        manager.capture(
            "api.response.end",
            serde_json::json!({
                "requestId": request_id,
                "duration": 1500,
                "usage": {
                    "inputTokens": 100,
                    "outputTokens": 50
                }
            }),
            None,
        );
    }

    // @step Then an "api.response.end" event should be written to the debug stream
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let response_event = events.iter().find(|e| e.event_type == "api.response.end");
    assert!(
        response_event.is_some(),
        "api.response.end event should exist"
    );
    let response_event = response_event.unwrap();

    // @step And the event should contain the same "requestId" as the request
    assert_eq!(
        response_event.data["requestId"].as_str().unwrap(),
        request_id,
        "requestId should match"
    );

    // @step And the event should contain the response duration in milliseconds
    assert_eq!(
        response_event.data["duration"].as_i64().unwrap(),
        1500,
        "duration should be 1500"
    );

    // @step And the event should contain token usage information
    assert!(
        response_event.data["usage"].is_object(),
        "usage should be defined"
    );
    assert_eq!(
        response_event.data["usage"]["inputTokens"]
            .as_i64()
            .unwrap(),
        100,
        "inputTokens should be 100"
    );
    assert_eq!(
        response_event.data["usage"]["outputTokens"]
            .as_i64()
            .unwrap(),
        50,
        "outputTokens should be 50"
    );
}

// ============================================================================
// Scenario: Capture tool call with arguments
// ============================================================================
#[test]
#[serial]
fn test_capture_tool_call_with_arguments() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step When the agent executes a tool call
    let tool_call_id = "tool-test-456";
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "tool.call",
            serde_json::json!({
                "toolCallId": tool_call_id,
                "toolName": "bash",
                "arguments": { "command": "ls -la" }
            }),
            None,
        );
    }

    // @step Then a "tool.call" event should be written to the debug stream
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tool_call_event = events.iter().find(|e| e.event_type == "tool.call");
    assert!(tool_call_event.is_some(), "tool.call event should exist");
    let tool_call_event = tool_call_event.unwrap();

    // @step And the event should contain the tool name
    assert_eq!(
        tool_call_event.data["toolName"].as_str().unwrap(),
        "bash",
        "toolName should be 'bash'"
    );

    // @step And the event should contain the tool arguments
    assert_eq!(
        tool_call_event.data["arguments"]["command"]
            .as_str()
            .unwrap(),
        "ls -la",
        "arguments.command should be 'ls -la'"
    );

    // @step And the event should contain a unique "toolCallId"
    assert_eq!(
        tool_call_event.data["toolCallId"].as_str().unwrap(),
        tool_call_id,
        "toolCallId should match"
    );
}

// ============================================================================
// Scenario: Capture tool result with timing
// ============================================================================
#[test]
#[serial]
fn test_capture_tool_result_with_timing() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step And a tool call was made with toolCallId "tool-456"
    let tool_call_id = "tool-456";
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "tool.call",
            serde_json::json!({
                "toolCallId": tool_call_id,
                "toolName": "bash",
                "arguments": { "command": "echo hello" }
            }),
            None,
        );

        // @step When the tool execution completes
        manager.capture(
            "tool.result",
            serde_json::json!({
                "toolCallId": tool_call_id,
                "toolName": "bash",
                "result": "hello\n",
                "duration": 50,
                "exitCode": 0
            }),
            None,
        );
    }

    // @step Then a "tool.result" event should be written to the debug stream
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tool_result_event = events.iter().find(|e| e.event_type == "tool.result");
    assert!(
        tool_result_event.is_some(),
        "tool.result event should exist"
    );
    let tool_result_event = tool_result_event.unwrap();

    // @step And the event should contain the same "toolCallId"
    assert_eq!(
        tool_result_event.data["toolCallId"].as_str().unwrap(),
        tool_call_id,
        "toolCallId should match"
    );

    // @step And the event should contain the execution duration in milliseconds
    assert_eq!(
        tool_result_event.data["duration"].as_i64().unwrap(),
        50,
        "duration should be 50"
    );

    // @step And the event should contain the exit code for bash tools
    assert_eq!(
        tool_result_event.data["exitCode"].as_i64().unwrap(),
        0,
        "exitCode should be 0"
    );
}

// ============================================================================
// Scenario: Merge tracing log entries into debug stream
// ============================================================================
#[test]
#[serial]
fn test_merge_tracing_log_entries_into_debug_stream() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step When the application logs an info message via tracing
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "log.entry",
            serde_json::json!({
                "level": "info",
                "message": "Test log message",
                "metadata": { "key": "value" }
            }),
            None,
        );
    }

    // @step Then a "log.entry" event should be written to the debug stream
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let log_event = events.iter().find(|e| e.event_type == "log.entry");
    assert!(log_event.is_some(), "log.entry event should exist");
    let log_event = log_event.unwrap();

    // @step And the event should contain the log level "info"
    assert_eq!(
        log_event.data["level"].as_str().unwrap(),
        "info",
        "level should be 'info'"
    );

    // @step And the event should contain the log message
    assert_eq!(
        log_event.data["message"].as_str().unwrap(),
        "Test log message",
        "message should match"
    );

    // @step And the event should have a timestamp synchronized with other events
    assert!(
        !log_event.timestamp.is_empty(),
        "timestamp should be defined"
    );
}

// ============================================================================
// Scenario: Generate session summary on capture stop
// ============================================================================
#[test]
#[serial]
fn test_generate_session_summary_on_capture_stop() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step And the session has recorded multiple events
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "user.input",
            serde_json::json!({ "input": "test command" }),
            None,
        );
        manager.capture(
            "api.request",
            serde_json::json!({ "requestId": "req-1", "headers": {} }),
            None,
        );
        manager.capture(
            "api.response.end",
            serde_json::json!({ "requestId": "req-1", "duration": 100 }),
            None,
        );
    }

    // @step When I stop debug capture with "/debug" command
    let stop_result = handle_debug_command();
    let summary_path = stop_result
        .session_file
        .as_ref()
        .unwrap()
        .replace(".jsonl", ".summary.md");

    // @step Then a summary markdown file should be generated
    assert!(
        PathBuf::from(&summary_path).exists(),
        "Summary file should exist"
    );

    // @step And the summary should contain session statistics
    let summary_content = fs::read_to_string(&summary_path).expect("Should read summary file");
    assert!(
        summary_content.contains("Session"),
        "Summary should contain 'Session'"
    );
    assert!(
        summary_content.contains("Statistics"),
        "Summary should contain 'Statistics'"
    );

    // @step And the summary should contain an event timeline
    assert!(
        summary_content.contains("Timeline"),
        "Summary should contain 'Timeline'"
    );

    // @step And the summary should list any errors or warnings
    assert!(
        summary_content.contains("Errors"),
        "Summary should contain 'Errors'"
    );
}

// ============================================================================
// Scenario: Redact sensitive credentials from captured headers
// ============================================================================
#[test]
#[serial]
fn test_redact_sensitive_credentials_from_captured_headers() {
    cleanup_manager();

    // @step Given debug capture is enabled
    handle_debug_command();

    // @step When the agent makes an API request with an authorization header
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "api.request",
            serde_json::json!({
                "requestId": "req-secret",
                "headers": {
                    "authorization": "Bearer sk-ant-api03-secret-key",
                    "x-api-key": "secret-api-key-12345",
                    "content-type": "application/json"
                },
                "body": {}
            }),
            None,
        );
    }

    // @step Then the captured "api.request" event should have headers
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let api_event = events.iter().find(|e| e.event_type == "api.request");
    assert!(
        api_event.unwrap().data["headers"].is_object(),
        "headers should be defined"
    );
    let api_event = api_event.unwrap();

    // @step And the "authorization" header value should be "[REDACTED]"
    assert_eq!(
        api_event.data["headers"]["authorization"].as_str().unwrap(),
        "[REDACTED]",
        "authorization should be redacted"
    );

    // @step And the "x-api-key" header value should be "[REDACTED]" if present
    assert_eq!(
        api_event.data["headers"]["x-api-key"].as_str().unwrap(),
        "[REDACTED]",
        "x-api-key should be redacted"
    );
}

// ============================================================================
// Scenario: Zero overhead when debug capture is disabled
// ============================================================================
#[test]
#[serial]
fn test_zero_overhead_when_debug_capture_is_disabled() {
    cleanup_manager();
    let test_debug_dir = get_test_debug_dir();

    // @step Given debug capture is disabled
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let manager = manager_arc.lock().expect("Failed to lock manager");
        assert!(!manager.is_enabled(), "Debug capture should be disabled");
    }

    // Count files in debug dir before
    let file_count_before = if test_debug_dir.exists() {
        fs::read_dir(&test_debug_dir).unwrap().count()
    } else {
        0
    };

    // @step When the agent processes multiple LLM requests and tool calls
    {
        let manager_arc = get_debug_capture_manager().unwrap();
        let mut manager = manager_arc.lock().expect("Failed to lock manager");
        manager.capture(
            "api.request",
            serde_json::json!({ "requestId": "req-1" }),
            None,
        );
        manager.capture(
            "api.response.end",
            serde_json::json!({ "requestId": "req-1" }),
            None,
        );
        manager.capture(
            "tool.call",
            serde_json::json!({ "toolCallId": "tool-1" }),
            None,
        );
        manager.capture(
            "tool.result",
            serde_json::json!({ "toolCallId": "tool-1" }),
            None,
        );

        // @step Then no debug events should be written
        // Events should be silently dropped when disabled
        assert!(
            !manager.is_enabled(),
            "Debug capture should still be disabled"
        );
    }

    // @step And no debug files should be created
    let file_count_after = if test_debug_dir.exists() {
        fs::read_dir(&test_debug_dir).unwrap().count()
    } else {
        0
    };
    assert_eq!(
        file_count_after, file_count_before,
        "No new files should be created"
    );

    // @step And the DebugCaptureManager should short-circuit all capture calls
    // This is implicitly tested by the fact that no files were created
}

// ============================================================================
// Scenario: Create debug directory with secure permissions
// ============================================================================
#[cfg(unix)]
#[test]
#[serial]
fn test_create_debug_directory_with_secure_permissions() {
    use std::os::unix::fs::PermissionsExt;

    cleanup_manager();
    let test_debug_dir = get_test_debug_dir();

    // @step Given the debug directory does not exist
    if test_debug_dir.exists() {
        fs::remove_dir_all(&test_debug_dir).expect("Should remove test debug dir");
    }
    assert!(!test_debug_dir.exists(), "Debug directory should not exist");

    // @step When debug capture is enabled for the first time
    handle_debug_command();

    // @step Then the directory "~/.codelet/debug/" should be created
    assert!(test_debug_dir.exists(), "Debug directory should exist");

    // @step And the directory should have permissions 0o700
    let metadata = fs::metadata(&test_debug_dir).expect("Should get metadata");
    let permissions = metadata.permissions().mode() & 0o777;
    assert_eq!(
        permissions, 0o700,
        "Directory should have 0o700 permissions"
    );

    // @step And the path should resolve correctly on macOS, Linux, and Windows
    // This is implicitly tested by using dirs::home_dir() which is cross-platform
    let home = dirs::home_dir().expect("Should get home dir");
    assert!(
        test_debug_dir.starts_with(&home),
        "Debug dir should be under home directory"
    );

    // Clean up
    handle_debug_command(); // stop capture
}

// ============================================================================
// Scenario: Record session start metadata
// ============================================================================
#[test]
#[serial]
fn test_record_session_start_metadata() {
    cleanup_manager();

    // @step Given the agent is running with provider "claude" and model "claude-sonnet-4"
    // (This is simulated - in real usage the provider would set this)

    // @step When debug capture is enabled
    handle_debug_command();

    // @step Then a "session.start" event should be written
    let stop_result = handle_debug_command();
    let content = fs::read_to_string(stop_result.session_file.as_ref().unwrap())
        .expect("Should read session file");
    let events: Vec<DebugEvent> = content
        .trim()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let session_start_event = events.iter().find(|e| e.event_type == "session.start");
    assert!(
        session_start_event.is_some(),
        "session.start event should exist"
    );
    let session_start_event = session_start_event.unwrap();

    // @step And the event should contain the provider name
    assert!(
        session_start_event.data.get("provider").is_some(),
        "provider should be defined"
    );

    // @step And the event should contain the model name
    assert!(
        session_start_event.data.get("model").is_some(),
        "model should be defined"
    );

    // @step And the event should contain environment information
    assert!(
        session_start_event.data.get("environment").is_some(),
        "environment should be defined"
    );
    let env = &session_start_event.data["environment"];
    assert!(env.get("platform").is_some(), "platform should be defined");
    assert!(env.get("arch").is_some(), "arch should be defined");

    // @step And the event should contain the context window size
    assert!(
        session_start_event.data.get("contextWindow").is_some(),
        "contextWindow should be defined"
    );
}
