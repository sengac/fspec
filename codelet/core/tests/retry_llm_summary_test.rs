
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/retry-llm-summary.feature
//!
//! Tests for LLM Summary Generation with Retry Logic
//!
//! The compactor uses LLM for both anchor detection AND summary generation.
//! These tests verify retry behavior and fallback handling.

use codelet_core::compaction::{ContextCompactor, ConversationTurn, ToolCall, ToolResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

// ==========================================
// TEST FIXTURES
// ==========================================

/// Create a simple test turn for compaction
fn create_test_turn(user_msg: &str, assistant_response: &str, tokens: u64) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: assistant_response.to_string(),
        timestamp: SystemTime::now(),
        tokens,
        previous_error: None,
    }
}

/// Create a turn with file modifications (for anchor detection)
fn create_turn_with_file_edit(
    user_msg: &str,
    assistant_response: &str,
    file_path: &str,
    tokens: u64,
) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        tool_calls: vec![ToolCall {
            tool: "Edit".to_string(),
            id: "call_1".to_string(),
            parameters: serde_json::json!({ "file_path": file_path }),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: format!("Edited file: {file_path}"),
            error: None,
        }],
        assistant_response: assistant_response.to_string(),
        timestamp: SystemTime::now(),
        tokens,
        previous_error: None,
    }
}

// ==========================================
// LLM SUMMARY GENERATION TESTS
// ==========================================

/// Scenario: Successful summary generation on first attempt
#[tokio::test]
async fn test_successful_summary_on_first_attempt() {
    // @step Given the LLM provider is functioning normally
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    let call_count_clone = call_count.clone();
    let llm_mock = move |prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            
            // Return appropriate response based on prompt type
            if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
                // Anchor detection prompt - return no anchors
                Ok::<String, anyhow::Error>(
                    r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
                )
            } else {
                // Summary generation prompt
                Ok("This is an LLM-generated summary of the conversation.".to_string())
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the summary should be generated successfully
    assert!(result.is_ok());
    let result = result.unwrap();

    // @step And the summary should contain LLM-generated content
    assert!(
        result.summary.contains("LLM-generated summary"),
        "Summary should be LLM-generated, got: {}",
        result.summary
    );

    // @step And LLM should be called (at least once for anchor detection, once for summary)
    assert!(
        call_count.load(Ordering::SeqCst) >= 2,
        "LLM should be called for both anchor detection and summary generation"
    );
}

/// Scenario: Retry on transient failure with eventual success
#[tokio::test]
async fn test_retry_on_transient_failure_with_eventual_success() {
    // @step Given the LLM provider fails on first attempt
    // @step And the LLM provider succeeds on second attempt
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    // Need 6+ turns so there are "older turns" to summarize (selector keeps last 3)
    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
        create_test_turn("Turn 5", "Response 5", 100),
        create_test_turn("Turn 6", "Response 6", 100),
    ];

    let summary_call_count_clone = summary_call_count.clone();
    let llm_mock = move |prompt: String| {
        let count = summary_call_count_clone.clone();
        async move {
            if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
                // Anchor detection - always succeed
                Ok::<String, anyhow::Error>(
                    r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
                )
            } else {
                // Summary generation - fail first, succeed second
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err(anyhow::anyhow!("Transient network error"))
                } else {
                    Ok("Summary after retry succeeded.".to_string())
                }
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the second attempt should succeed
    assert!(result.is_ok());
    let result = result.unwrap();

    // @step And the summary should be returned
    assert!(
        result.summary.contains("retry succeeded"),
        "Summary should be from successful retry, got: {}",
        result.summary
    );

    // @step And at least 2 summary attempts should have been made
    assert!(
        summary_call_count.load(Ordering::SeqCst) >= 2,
        "Should have retried after first failure"
    );
}

/// Scenario: Fallback behavior when all retries fail
#[tokio::test]
async fn test_fallback_when_all_retries_fail() {
    // @step Given the LLM provider fails on all retry attempts
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    // Need 6+ turns so there are "older turns" to summarize (selector keeps last 3)
    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
        create_test_turn("Turn 5", "Response 5", 100),
        create_test_turn("Turn 6", "Response 6", 100),
    ];

    let summary_call_count_clone = summary_call_count.clone();
    let llm_mock = move |prompt: String| {
        let count = summary_call_count_clone.clone();
        async move {
            if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
                // Anchor detection - always succeed
                Ok::<String, anyhow::Error>(
                    r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
                )
            } else {
                // Summary generation - always fail
                count.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::anyhow!("Persistent LLM failure"))
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then compaction should not fail entirely
    assert!(result.is_ok());

    // @step And a fallback summary should be generated
    let result = result.unwrap();
    assert!(
        result.summary.contains("Summary generation failed"),
        "Should use fallback summary when all retries fail, got: {}",
        result.summary
    );

    // @step And kept messages should still be returned
    assert!(!result.kept_turns.is_empty() || result.metrics.turns_summarized > 0);

    // @step And 3 retry attempts should have been made
    assert_eq!(
        summary_call_count.load(Ordering::SeqCst),
        3,
        "Should have made 3 retry attempts"
    );
}

/// Scenario: Summary includes anchor markers for important turns
#[tokio::test]
async fn test_summary_includes_anchor_markers() {
    // @step Given turns with file modifications (anchor-worthy)
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);

    let turns = vec![
        create_test_turn("Start", "Beginning work", 100),
        create_turn_with_file_edit("Edit file", "Fixed the bug in lib.rs", "src/lib.rs", 200),
        create_test_turn("Continue", "Continuing with next task", 100),
        create_test_turn("Final", "All done", 100),
    ];

    let llm_mock = |prompt: String| async move {
        if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
            // Return anchor for the file edit turn
            Ok::<String, anyhow::Error>(
                r#"[
                    {"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"},
                    {"turn_index": 1, "anchor_type": "TaskCompletion", "confidence": 0.95, "description": "Fixed bug"},
                    {"turn_index": 2, "anchor_type": null, "confidence": 0.0, "description": "No anchor"},
                    {"turn_index": 3, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}
                ]"#.to_string()
            )
        } else {
            Ok("Summary with anchor: Fixed bug in lib.rs [ANCHOR].".to_string())
        }
    };

    // @step When compaction is triggered
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the result should succeed
    assert!(result.is_ok());

    // @step And anchor points should be detected
    let result = result.unwrap();
    assert!(result.anchor.is_some(), "Should have detected anchor point");
}

/// Scenario: Empty turns return error
#[tokio::test]
async fn test_empty_turns_returns_error() {
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns: Vec<ConversationTurn> = vec![];

    let call_count_clone = call_count.clone();
    let llm_mock = move |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok::<String, anyhow::Error>("Should not be called".to_string())
        }
    };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // Empty turns should fail (cannot compact empty history)
    assert!(result.is_err());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        0,
        "LLM should not be called for empty turns"
    );
}

/// Scenario: Compression metrics are calculated correctly
#[tokio::test]
async fn test_compression_metrics_calculated() {
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);

    let turns = vec![
        create_test_turn("Turn 1", "Response 1 with some content", 500),
        create_test_turn("Turn 2", "Response 2 with more content", 500),
        create_test_turn("Turn 3", "Response 3 with even more", 500),
        create_test_turn("Turn 4", "Response 4 final content", 500),
    ];

    let llm_mock = |prompt: String| async move {
        if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
            Ok::<String, anyhow::Error>(
                r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
            )
        } else {
            Ok("Concise summary.".to_string())
        }
    };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Metrics should be calculated
    assert!(result.metrics.original_tokens > 0);
    assert!(result.metrics.turns_summarized + result.metrics.turns_kept == turns.len());
}

/// Scenario: Kept turns are preserved correctly
#[tokio::test]
async fn test_kept_turns_preserved() {
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);

    let turns = vec![
        create_test_turn("Old turn 1", "Old response 1", 100),
        create_test_turn("Old turn 2", "Old response 2", 100),
        create_test_turn("Recent turn", "Recent response", 100),
        create_test_turn("Latest turn", "Latest response", 100),
    ];

    let llm_mock = |prompt: String| async move {
        if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
            Ok::<String, anyhow::Error>(
                r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
            )
        } else {
            Ok("Summary of old turns.".to_string())
        }
    };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should have kept some turns
    assert!(!result.kept_turns.is_empty(), "Should preserve some turns");
}
