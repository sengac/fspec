//! Comprehensive test proving all 17 debug capture event types work
//!
//! This example captures ALL 17 event types from codelet's debug-capture.ts
//! and verifies each one is written to the JSONL file.

use codelet::debug_capture::{
    get_debug_capture_manager, handle_debug_command, CaptureOptions, DebugEvent, SessionMetadata,
};
use std::collections::HashSet;
use std::fs;

fn main() {
    println!("=== Testing ALL 17 Debug Capture Event Types ===\n");

    // 1. Enable debug capture
    println!("1. Enabling debug capture...");
    let result = handle_debug_command();
    assert!(result.enabled, "Debug should be enabled");
    let session_file = result.session_file.clone().unwrap();
    println!("   Session file: {}\n", session_file);

    // 2. Set session metadata
    println!("2. Setting session metadata...");
    {
        let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
        let mut manager = manager_arc.lock().expect("Failed to lock");
        manager.set_session_metadata(SessionMetadata {
            provider: Some("test-provider".to_string()),
            model: Some("test-model".to_string()),
            context_window: Some(200000),
            max_output_tokens: Some(16384),
        });
    }
    println!("   ✓ Metadata set\n");

    // 3. Capture all 17 event types
    println!("3. Capturing all 17 event types...\n");
    {
        let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
        let mut manager = manager_arc.lock().expect("Failed to lock");

        // Note: session.start (1) is already captured when we enabled debug

        // 2. api.request
        manager.capture(
            "api.request",
            serde_json::json!({
                "provider": "claude",
                "model": "claude-sonnet-4",
                "headers": {
                    "authorization": "Bearer sk-secret-key",
                    "content-type": "application/json"
                },
                "prompt": "Hello, world!"
            }),
            Some(CaptureOptions {
                request_id: Some("req-001".to_string()),
            }),
        );
        println!("   ✓ 2. api.request");

        // 3. api.response.start
        manager.capture(
            "api.response.start",
            serde_json::json!({
                "provider": "claude"
            }),
            Some(CaptureOptions {
                request_id: Some("req-001".to_string()),
            }),
        );
        println!("   ✓ 3. api.response.start");

        // 4. api.response.chunk
        manager.capture(
            "api.response.chunk",
            serde_json::json!({
                "chunkLength": 50
            }),
            Some(CaptureOptions {
                request_id: Some("req-001".to_string()),
            }),
        );
        println!("   ✓ 4. api.response.chunk");

        // 5. api.response.end
        manager.capture(
            "api.response.end",
            serde_json::json!({
                "duration": 1500,
                "usage": {
                    "inputTokens": 100,
                    "outputTokens": 50
                }
            }),
            Some(CaptureOptions {
                request_id: Some("req-001".to_string()),
            }),
        );
        println!("   ✓ 5. api.response.end");

        // 6. api.error
        manager.capture(
            "api.error",
            serde_json::json!({
                "error": "Rate limit exceeded",
                "statusCode": 429,
                "duration": 500
            }),
            Some(CaptureOptions {
                request_id: Some("req-002".to_string()),
            }),
        );
        println!("   ✓ 6. api.error");

        // 7. tool.call
        manager.capture(
            "tool.call",
            serde_json::json!({
                "toolName": "Bash",
                "toolId": "tool-001",
                "arguments": {"command": "ls -la"}
            }),
            None,
        );
        println!("   ✓ 7. tool.call");

        // 8. tool.result
        manager.capture(
            "tool.result",
            serde_json::json!({
                "toolName": "Bash",
                "toolId": "tool-001",
                "success": true,
                "duration": 150
            }),
            None,
        );
        println!("   ✓ 8. tool.result");

        // 9. tool.error
        manager.capture(
            "tool.error",
            serde_json::json!({
                "toolName": "Bash",
                "toolId": "tool-002",
                "error": "Command not found",
                "exitCode": 127
            }),
            None,
        );
        println!("   ✓ 9. tool.error");

        // 10. context.update
        manager.capture(
            "context.update",
            serde_json::json!({
                "type": "compaction",
                "originalTokens": 50000,
                "compactedTokens": 10000,
                "compressionRatio": 0.8
            }),
            None,
        );
        println!("   ✓ 10. context.update");

        // 11. token.update
        manager.capture(
            "token.update",
            serde_json::json!({
                "inputTokens": 100,
                "outputTokens": 50,
                "totalInputTokens": 1000,
                "totalOutputTokens": 500
            }),
            None,
        );
        println!("   ✓ 11. token.update");

        // 12. compaction.triggered
        manager.capture(
            "compaction.triggered",
            serde_json::json!({
                "effectiveTokens": 180000,
                "threshold": 160000,
                "contextWindow": 200000
            }),
            None,
        );
        println!("   ✓ 12. compaction.triggered");

        // 13. provider.switch
        manager.capture(
            "provider.switch",
            serde_json::json!({
                "from": "claude",
                "to": "openai"
            }),
            None,
        );
        println!("   ✓ 13. provider.switch");

        // 14. log.entry
        manager.capture(
            "log.entry",
            serde_json::json!({
                "level": "info",
                "message": "Agent started processing",
                "target": "codelet::agent"
            }),
            None,
        );
        println!("   ✓ 14. log.entry");

        // 15. user.input
        manager.capture(
            "user.input",
            serde_json::json!({
                "input": "Help me fix this bug",
                "inputLength": 21
            }),
            None,
        );
        println!("   ✓ 15. user.input");

        // 16. command.executed
        manager.capture(
            "command.executed",
            serde_json::json!({
                "command": "/debug",
                "result": "enabled"
            }),
            None,
        );
        println!("   ✓ 16. command.executed");

        // Note: session.end (17) will be captured when we disable debug
    }

    // 4. Disable debug capture (this captures session.end)
    println!("\n4. Disabling debug capture (captures session.end)...");
    let result = handle_debug_command();
    assert!(!result.enabled, "Debug should be disabled");
    println!("   ✓ 17. session.end\n");

    // 5. Verify all 17 event types are in the file
    println!("5. Verifying all 17 event types in session file...\n");
    let content = fs::read_to_string(&session_file).expect("Failed to read session file");
    let lines: Vec<&str> = content.trim().lines().collect();

    println!("   Total events captured: {}\n", lines.len());

    // Collect all event types
    let mut event_types: HashSet<String> = HashSet::new();
    for line in &lines {
        let event: DebugEvent = serde_json::from_str(line).expect("Invalid JSON");
        event_types.insert(event.event_type.clone());
    }

    // The 17 expected event types
    let expected_types = vec![
        "session.start",
        "session.end",
        "api.request",
        "api.response.start",
        "api.response.chunk",
        "api.response.end",
        "api.error",
        "tool.call",
        "tool.result",
        "tool.error",
        "context.update",
        "token.update",
        "compaction.triggered",
        "provider.switch",
        "log.entry",
        "user.input",
        "command.executed",
    ];

    println!("   Event types found:");
    let mut all_found = true;
    for (i, expected) in expected_types.iter().enumerate() {
        let found = event_types.contains(*expected);
        let status = if found { "✓" } else { "✗" };
        println!("   {} {:2}. {}", status, i + 1, expected);
        if !found {
            all_found = false;
        }
    }

    println!();

    // 6. Verify credential redaction
    println!("6. Verifying credential redaction...");
    for line in &lines {
        let event: serde_json::Value = serde_json::from_str(line).unwrap();
        if event["eventType"] == "api.request" {
            let auth = event["data"]["headers"]["authorization"]
                .as_str()
                .unwrap_or("");
            assert_eq!(auth, "[REDACTED]", "Authorization should be redacted");
            println!("   ✓ Authorization header redacted: {}", auth);
        }
    }
    println!();

    // 7. Final result
    if all_found {
        println!("=== ALL 17 EVENT TYPES VERIFIED ===");
        println!("\nSession file: {}", session_file);
    } else {
        println!("=== SOME EVENT TYPES MISSING ===");
        std::process::exit(1);
    }
}
