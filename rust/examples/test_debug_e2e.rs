//! Quick test to prove debug capture works end-to-end

use codelet::debug_capture::{
    get_debug_capture_manager, handle_debug_command, CaptureOptions, SessionMetadata,
};
use std::fs;

fn main() {
    println!("=== Debug Capture End-to-End Test ===\n");

    // 1. Enable debug capture
    println!("1. Enabling debug capture with /debug command...");
    let result = handle_debug_command();
    println!("   Result: {}", result.message);
    println!("   Enabled: {}", result.enabled);
    assert!(result.enabled, "Debug should be enabled");
    
    let session_file = result.session_file.clone().unwrap();
    println!("   Session file: {}", session_file);
    
    // Verify file was created
    assert!(std::path::Path::new(&session_file).exists(), "Session file should exist");
    println!("   ✓ Session file exists\n");

    // 2. Set session metadata
    println!("2. Setting session metadata...");
    {
        let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
        let mut manager = manager_arc.lock().expect("Failed to lock");
        manager.set_session_metadata(SessionMetadata {
            provider: Some("test-provider".to_string()),
            model: Some("test-model".to_string()),
            context_window: Some(100000),
            max_output_tokens: Some(8192),
        });
        println!("   ✓ Metadata set\n");
    }

    // 3. Capture some events
    println!("3. Capturing events...");
    {
        let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
        let mut manager = manager_arc.lock().expect("Failed to lock");
        
        // Capture API request
        manager.capture(
            "api.request",
            serde_json::json!({
                "provider": "test-provider",
                "model": "test-model",
                "headers": {
                    "authorization": "Bearer sk-secret-key",
                    "content-type": "application/json"
                },
                "payload": {"messages": [{"role": "user", "content": "Hello"}]}
            }),
            Some(CaptureOptions { request_id: Some("req-001".to_string()) }),
        );
        println!("   ✓ Captured api.request");

        // Capture tool call
        manager.capture(
            "tool.call",
            serde_json::json!({
                "toolName": "Bash",
                "toolId": "tool-001",
                "arguments": {"command": "ls -la"}
            }),
            None,
        );
        println!("   ✓ Captured tool.call");

        // Capture tool result
        manager.capture(
            "tool.result",
            serde_json::json!({
                "toolName": "Bash",
                "toolId": "tool-001",
                "success": true,
                "output": "total 0\ndrwxr-xr-x 2 user user 40 Jan 1 00:00 .",
                "duration": 150
            }),
            None,
        );
        println!("   ✓ Captured tool.result");

        // Capture API response
        manager.capture(
            "api.response.end",
            serde_json::json!({
                "duration": 1500,
                "usage": {
                    "inputTokens": 100,
                    "outputTokens": 50
                }
            }),
            Some(CaptureOptions { request_id: Some("req-001".to_string()) }),
        );
        println!("   ✓ Captured api.response.end\n");
    }

    // 4. Disable debug capture
    println!("4. Disabling debug capture with /debug command...");
    let result = handle_debug_command();
    println!("   Result: {}", result.message);
    println!("   Enabled: {}", result.enabled);
    assert!(!result.enabled, "Debug should be disabled");
    println!("   ✓ Debug disabled\n");

    // 5. Verify JSONL file contents
    println!("5. Verifying session file contents...");
    let content = fs::read_to_string(&session_file).expect("Failed to read session file");
    let lines: Vec<&str> = content.trim().lines().collect();
    println!("   Total events captured: {}", lines.len());
    
    // Parse and display events
    for (i, line) in lines.iter().enumerate() {
        let event: serde_json::Value = serde_json::from_str(line).expect("Invalid JSON");
        let event_type = event["eventType"].as_str().unwrap_or("unknown");
        let seq = event["sequence"].as_u64().unwrap_or(0);
        println!("   [{}] seq={} type={}", i, seq, event_type);
        
        // Verify credential redaction
        if event_type == "api.request" {
            let auth = event["data"]["headers"]["authorization"].as_str().unwrap_or("");
            assert_eq!(auth, "[REDACTED]", "Authorization header should be redacted");
            println!("       ✓ Authorization header redacted");
        }
    }
    println!();

    // 6. Verify summary file
    let summary_file = session_file.replace(".jsonl", ".summary.md");
    println!("6. Verifying summary file...");
    assert!(std::path::Path::new(&summary_file).exists(), "Summary file should exist");
    let summary = fs::read_to_string(&summary_file).expect("Failed to read summary");
    assert!(summary.contains("Debug Session Summary"), "Summary should have header");
    assert!(summary.contains("Events:"), "Summary should have event count");
    println!("   Summary file: {}", summary_file);
    println!("   ✓ Summary file exists and valid\n");

    // 7. Verify zero overhead when disabled
    println!("7. Testing zero overhead when disabled...");
    {
        let manager_arc = get_debug_capture_manager().expect("Failed to get manager");
        let mut manager = manager_arc.lock().expect("Failed to lock");
        
        // This should do nothing (no file I/O)
        let start = std::time::Instant::now();
        for _ in 0..10000 {
            manager.capture("test.event", serde_json::json!({"test": true}), None);
        }
        let elapsed = start.elapsed();
        println!("   10,000 capture() calls when disabled: {:?}", elapsed);
        assert!(elapsed.as_millis() < 10, "Should be nearly instant when disabled");
        println!("   ✓ Zero overhead confirmed\n");
    }

    println!("=== ALL TESTS PASSED ===");
}
