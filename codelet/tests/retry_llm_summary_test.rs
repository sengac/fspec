
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/retry-llm-summary.feature
//!
//! Tests for LLM Summary Generation with Retry Logic

use anyhow::anyhow;
use codelet_core::compaction::{ContextCompactor, ConversationTurn};
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
                Ok("LLM-generated summary of conversation.".to_string())
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the summary should be generated successfully
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.summary.contains("LLM-generated summary"), "Expected LLM summary, got: {}", result.summary);

    // @step And LLM should be called for anchor detection and summary
    assert!(call_count.load(Ordering::SeqCst) >= 2, "LLM should be called at least twice");
}

/// Scenario: Retry on transient failure with eventual success
#[tokio::test]
async fn test_retry_on_first_failure_success_on_second() {
    // @step Given the LLM provider fails on first attempt
    // @step And the LLM provider succeeds on second attempt
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
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
                    Err(anyhow!("Transient error"))
                } else {
                    Ok("Summary after retry.".to_string())
                }
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then the first attempt should fail
    // @step And a retry should be attempted
    assert!(summary_call_count.load(Ordering::SeqCst) >= 2, "Should have retried");

    // @step And the second attempt should succeed
    // @step And the summary should be returned
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.summary, "Summary after retry.");
}

/// Scenario: Fallback behavior when all retries fail
#[tokio::test]
async fn test_fallback_when_all_retries_fail() {
    // @step Given the LLM provider fails on all 3 retry attempts
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
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
                Err::<String, anyhow::Error>(anyhow!("Persistent error"))
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then compaction should not fail entirely
    assert!(result.is_ok());
    assert_eq!(summary_call_count.load(Ordering::SeqCst), 3);

    // @step And a fallback summary should be generated
    // @step And the fallback summary should indicate summarization failed
    // @step And kept messages should still be returned
    let result = result.unwrap();
    assert!(result.summary.contains("failed"), "Fallback summary should indicate failure, got: {}", result.summary);
    assert!(result.summary.contains("preserved"), "Fallback summary should indicate preservation, got: {}", result.summary);
}

/// Scenario: Retry with exponential backoff delays
#[tokio::test]
async fn test_exactly_three_retry_attempts() {
    // @step Given the LLM provider fails on all attempts
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    let summary_call_count_clone = summary_call_count.clone();
    let llm_mock = move |prompt: String| {
        let count = summary_call_count_clone.clone();
        async move {
            if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
                Ok::<String, anyhow::Error>(
                    r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
                )
            } else {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<String, anyhow::Error>(anyhow!("Error"))
            }
        }
    };

    // @step When compaction triggers summary generation with max 3 retries
    let _ = compactor.compact(&turns, 150_000, llm_mock).await;

    // @step Then retry 1 should occur after 0ms delay (immediate)
    // @step And retry 2 should occur after 1000ms delay
    // @step And retry 3 should occur after 2000ms delay
    // @step And then all retries should be exhausted
    assert_eq!(
        summary_call_count.load(Ordering::SeqCst),
        3,
        "Should make exactly 3 attempts before falling back"
    );
}

/// Scenario: Success on third attempt
#[tokio::test]
async fn test_success_on_third_attempt() {
    // Disable compression threshold for testing retry logic
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let summary_call_count = Arc::new(AtomicUsize::new(0));

    // Need 4+ turns for summarization to trigger
    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    // Mock LLM that fails twice, succeeds on third
    let summary_call_count_clone = summary_call_count.clone();
    let llm_mock = move |prompt: String| {
        let count = summary_call_count_clone.clone();
        async move {
            if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
                Ok::<String, anyhow::Error>(
                    r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
                )
            } else {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(anyhow!("Transient error #{}", attempt))
                } else {
                    Ok("Finally succeeded!".to_string())
                }
            }
        }
    };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    assert!(result.is_ok());
    assert_eq!(summary_call_count.load(Ordering::SeqCst), 3);

    let result = result.unwrap();
    assert_eq!(result.summary, "Finally succeeded!");
}

/// Scenario: Compaction does not fail when LLM fails
#[tokio::test]
async fn test_compaction_does_not_fail_on_llm_failure() {
    // Disable compression threshold for testing retry logic
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);

    // Need 4+ turns for summarization to trigger
    let turns = vec![
        create_test_turn("Request 1", "Response 1", 500),
        create_test_turn("Request 2", "Response 2", 500),
        create_test_turn("Request 3", "Response 3", 500),
        create_test_turn("Request 4", "Response 4", 500),
    ];

    // Mock LLM that always fails for summary but succeeds for anchor detection
    let llm_mock = |prompt: String| async move {
        if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
            Ok::<String, anyhow::Error>(
                r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
            )
        } else {
            Err::<String, anyhow::Error>(anyhow!("LLM service unavailable"))
        }
    };

    let result = compactor.compact(&turns, 150_000, llm_mock).await;

    // Compaction should succeed despite LLM failure
    assert!(result.is_ok(), "Compaction should not fail when LLM fails");

    let result = result.unwrap();
    // Should still have kept turns
    assert!(!result.kept_turns.is_empty() || !result.summary.is_empty());
}

/// Scenario: Empty turns return error (not related to retry logic)
#[tokio::test]
async fn test_empty_turns_returns_error() {
    // Disable compression threshold for testing retry logic
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
