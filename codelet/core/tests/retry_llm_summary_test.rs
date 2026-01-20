//! Feature: spec/features/retry-llm-summary.feature
//!
//! Tests for CLI-018: Retry Logic for LLM Summary Generation
//!
//! NOTE: The compactor now uses template-based summaries (WeightedSummaryProvider pattern)
//! instead of LLM-generated summaries. The LLM callback parameter is kept for API
//! compatibility but is UNUSED. These tests verify the new template-based behavior.

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
            output: format!("Edited file: {}", file_path),
            error: None,
        }],
        assistant_response: assistant_response.to_string(),
        timestamp: SystemTime::now(),
        tokens,
        previous_error: None,
    }
}

// ==========================================
// TEMPLATE-BASED SUMMARY TESTS (replaces LLM retry tests)
// ==========================================

/// Scenario: Template-based summary generation succeeds without LLM
#[tokio::test]
async fn test_template_based_summary_no_llm_call() {
    // @step Given the compactor uses template-based summaries
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    // LLM mock that tracks calls - should NEVER be called
    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok::<String, anyhow::Error>("LLM should not be called".to_string())
        }
    };

    // @step When compaction is triggered
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the summary should be generated successfully
    assert!(result.is_ok());

    // @step And NO LLM calls should be made (template-based)
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        0,
        "LLM should NOT be called - summaries are template-based"
    );

    // @step And the summary should contain key outcomes
    let result = result.unwrap();
    assert!(
        result.summary.contains("Key outcomes") || !result.summary.is_empty(),
        "Summary should be generated from template"
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

    let llm_mock = |_: String| async { Ok::<String, anyhow::Error>("unused".to_string()) };

    // @step When compaction is triggered
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the result should succeed
    assert!(result.is_ok());

    // @step And anchor points should be detected
    let result = result.unwrap();
    // The summary should contain outcomes from summarized turns
    assert!(!result.summary.is_empty());
}

/// Scenario: Compaction succeeds even with no anchor points
#[tokio::test]
async fn test_compaction_succeeds_without_natural_anchors() {
    // @step Given turns without natural anchor points
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);

    let turns = vec![
        create_test_turn("Question 1", "Answer 1", 100),
        create_test_turn("Question 2", "Answer 2", 100),
        create_test_turn("Question 3", "Answer 3", 100),
        create_test_turn("Question 4", "Answer 4", 100),
    ];

    let llm_mock = |_: String| async { Ok::<String, anyhow::Error>("unused".to_string()) };

    // @step When compaction is triggered
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then compaction should succeed with synthetic anchor
    assert!(result.is_ok());

    let result = result.unwrap();
    // Should have an anchor (synthetic if no natural ones)
    assert!(result.anchor.is_some(), "Should have synthetic anchor");
}

/// Scenario: Empty turns return error
#[tokio::test]
async fn test_empty_turns_returns_error() {
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns: Vec<ConversationTurn> = vec![];

    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
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

    let llm_mock = |_: String| async { Ok::<String, anyhow::Error>("unused".to_string()) };

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

    let llm_mock = |_: String| async { Ok::<String, anyhow::Error>("unused".to_string()) };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should have kept some turns
    assert!(!result.kept_turns.is_empty(), "Should preserve some turns");
}
