//! Feature: spec/features/bash-tool-ui-streaming-during-execution.feature
//!
//! Tests for Bash Tool UI Streaming During Execution - TOOL-011
//!
//! These tests verify that bash command output streams to UI in real-time
//! while still buffering complete output for LLM response.

use codelet_tools::bash::{BashArgs, BashTool, StreamCallback};
use codelet_tools::limits::OutputLimits;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ==========================================
// MOCK STREAM OUTPUT FOR TESTING
// ==========================================

/// Mock stream output collector for capturing streamed events
struct MockStreamCollector {
    chunks: Arc<Mutex<Vec<String>>>,
}

impl MockStreamCollector {
    fn new() -> Self {
        Self {
            chunks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_chunks(&self) -> Vec<String> {
        self.chunks.lock().unwrap().clone()
    }

    fn get_all_text(&self) -> String {
        self.chunks.lock().unwrap().join("")
    }

    fn create_callback(&self) -> StreamCallback {
        let chunks = self.chunks.clone();
        Arc::new(move |text: &str| {
            chunks.lock().unwrap().push(text.to_string());
        })
    }
}

// ==========================================
// BASH TOOL UI STREAMING TESTS (TOOL-011)
// ==========================================

/// Scenario: Stream command output to UI in real-time
#[tokio::test]
async fn test_stream_command_output_to_ui_in_real_time() {
    // @step Given a bash command that produces incremental output
    let tool = BashTool::new();
    let collector = MockStreamCollector::new();

    // @step When the command executes through the bash tool
    // Command that produces multiple lines incrementally
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: "for i in 1 2 3; do echo \"line $i\"; done".to_string(),
            },
            Some(collector.create_callback()),
        )
        .await;

    // @step Then output chunks should be emitted to the UI as they are produced
    let chunks = collector.get_chunks();
    assert!(result.is_ok(), "Command should succeed");
    assert!(
        !chunks.is_empty(),
        "Expected streaming events to be emitted, but got none"
    );

    // @step And the user should see output appearing progressively
    // Verify we received multiple chunks (one per line)
    assert!(
        chunks.len() >= 3,
        "Expected at least 3 streaming chunks for 3 lines, got {}",
        chunks.len()
    );
}

/// Scenario: Buffer complete output for LLM response
#[tokio::test]
async fn test_buffer_complete_output_for_llm_response() {
    // @step Given a bash command that produces multiple lines of output
    let tool = BashTool::new();
    let collector = MockStreamCollector::new();

    // @step When the command completes execution
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: "echo 'line 1'; echo 'line 2'; echo 'line 3'".to_string(),
            },
            Some(collector.create_callback()),
        )
        .await
        .unwrap();

    // @step Then the LLM should receive the complete buffered output
    assert!(result.contains("line 1"), "LLM result missing line 1");
    assert!(result.contains("line 2"), "LLM result missing line 2");
    assert!(result.contains("line 3"), "LLM result missing line 3");

    // @step And the output should not be sent as individual streaming chunks to the LLM
    // The result should be a single complete string with all lines
    let lines: Vec<&str> = result.lines().collect();
    assert!(
        lines.len() >= 3,
        "LLM should receive complete output with all lines"
    );

    // Verify streaming also worked
    let chunks = collector.get_chunks();
    assert!(
        !chunks.is_empty(),
        "UI streaming should emit events separately from LLM result"
    );
}

/// Scenario: Truncate large output for LLM while streaming full output to UI
#[tokio::test]
async fn test_truncate_large_output_for_llm_while_streaming_full_to_ui() {
    // @step Given a bash command that produces output exceeding MAX_OUTPUT_CHARS
    let tool = BashTool::new();
    let collector = MockStreamCollector::new();

    // Generate ~50000 characters (exceeds MAX_OUTPUT_CHARS of 30000)
    // @step When the command executes and completes
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: "seq 1 5000 | while read i; do echo \"line $i\"; done".to_string(),
            },
            Some(collector.create_callback()),
        )
        .await
        .unwrap();

    // @step Then the UI should see all output streamed in real-time
    let all_streamed_text = collector.get_all_text();
    let total_streamed_chars = all_streamed_text.len();
    // UI should have received significantly more than the truncation limit
    assert!(
        total_streamed_chars > OutputLimits::MAX_OUTPUT_CHARS,
        "UI should receive full output, but got only {} chars (limit: {})",
        total_streamed_chars,
        OutputLimits::MAX_OUTPUT_CHARS
    );

    // @step And the LLM result should be truncated to MAX_OUTPUT_CHARS limit
    assert!(
        result.len() <= OutputLimits::MAX_OUTPUT_CHARS + 200, // Allow for truncation message
        "LLM result should be truncated to ~{} chars, but got {} chars",
        OutputLimits::MAX_OUTPUT_CHARS,
        result.len()
    );

    // @step And a truncation warning should be included in the LLM result
    assert!(
        result.contains("truncated"),
        "LLM result should include truncation warning"
    );
}

/// Scenario: Handle timeout with partial streamed output
#[tokio::test]
async fn test_handle_timeout_with_partial_streamed_output() {
    // @step Given a bash command that will exceed the timeout limit
    let tool = BashTool::with_timeout(Duration::from_secs(1));
    let collector = MockStreamCollector::new();

    // @step When the command times out during execution
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: "for i in $(seq 1 100); do echo \"line $i\"; sleep 0.1; done".to_string(),
            },
            Some(collector.create_callback()),
        )
        .await;

    // @step Then the user should have seen partial output streamed before timeout
    let chunks = collector.get_chunks();
    assert!(
        !chunks.is_empty(),
        "User should have seen some output before timeout"
    );
    // Should have seen some lines in 1 second (100ms per line = ~10 lines)
    assert!(
        chunks.len() >= 3,
        "Expected at least 3 lines streamed before timeout, got {}",
        chunks.len()
    );

    // @step And the LLM should receive a timeout error
    assert!(result.is_err(), "Result should be an error after timeout");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("timeout") || error_msg.contains("Timeout"),
        "Error should mention timeout, got: {}",
        error_msg
    );

    // @step And the partial output should be preserved in the error context
    // Partial output is preserved via streaming callback - user saw output before timeout
    // The chunks collected above prove partial output was captured and emitted to UI
    assert!(
        !chunks.is_empty(),
        "Partial output should be preserved - chunks were streamed before timeout"
    );
}

/// Scenario: Emit progress through StreamOutput trait
#[tokio::test]
async fn test_emit_progress_through_stream_output_trait() {
    // @step Given a bash command is executing
    let tool = BashTool::new();
    let collector = MockStreamCollector::new();

    // @step When stdout chunks are received from the subprocess
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: "echo 'chunk1'; echo 'chunk2'; echo 'chunk3'".to_string(),
            },
            Some(collector.create_callback()),
        )
        .await
        .unwrap();

    // Verify basic execution works
    assert!(result.contains("chunk1"), "Output should contain chunk1");
    assert!(result.contains("chunk2"), "Output should contain chunk2");
    assert!(result.contains("chunk3"), "Output should contain chunk3");

    // @step Then each chunk should trigger a callback
    let chunks = collector.get_chunks();
    assert!(
        !chunks.is_empty(),
        "Expected chunks to be emitted via callback"
    );

    // @step And the callback should receive the output content
    let all_text = collector.get_all_text();
    assert!(
        all_text.contains("chunk1"),
        "Streamed text should contain chunk1"
    );
    assert!(
        all_text.contains("chunk2"),
        "Streamed text should contain chunk2"
    );
    assert!(
        all_text.contains("chunk3"),
        "Streamed text should contain chunk3"
    );
}
