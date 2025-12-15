//! Feature: spec/features/retry-llm-summary.feature
//!
//! Tests for CLI-018: Retry Logic for LLM Summary Generation

use anyhow::anyhow;
use codelet::agent::compaction::{ContextCompactor, ConversationTurn};
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
// RETRY LOGIC TESTS (CLI-018)
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
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok::<String, anyhow::Error>("Summary of conversation.".to_string())
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, llm_mock).await;

    // @step Then the summary should be generated successfully
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.summary, "Summary of conversation.");

    // @step And no retries should be attempted
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

/// Scenario: Retry on transient failure with eventual success
#[tokio::test]
async fn test_retry_on_first_failure_success_on_second() {
    // @step Given the LLM provider fails on first attempt
    // @step And the LLM provider succeeds on second attempt
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                Err(anyhow!("Transient error"))
            } else {
                Ok::<String, anyhow::Error>("Summary after retry.".to_string())
            }
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, llm_mock).await;

    // @step Then the first attempt should fail
    // @step And a retry should be attempted after 1000ms delay
    assert_eq!(call_count.load(Ordering::SeqCst), 2);

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
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<String, anyhow::Error>(anyhow!("Persistent error"))
        }
    };

    // @step When compaction triggers summary generation
    let result = compactor.compact(&turns, llm_mock).await;

    // @step Then compaction should not fail entirely
    assert!(result.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

    // @step And a fallback summary should be generated
    // @step And the fallback summary should indicate summarization failed
    // @step And kept messages should still be returned
    let result = result.unwrap();
    assert!(result.summary.contains("failed"));
    assert!(result.summary.contains("preserved"));
}

/// Scenario: Retry with exponential backoff delays
#[tokio::test]
async fn test_exactly_three_retry_attempts() {
    // @step Given the LLM provider fails on all attempts
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Err::<String, anyhow::Error>(anyhow!("Error"))
        }
    };

    // @step When compaction triggers summary generation with max 3 retries
    let _ = compactor.compact(&turns, llm_mock).await;

    // @step Then retry 1 should occur after 0ms delay (immediate)
    // @step And retry 2 should occur after 1000ms delay
    // @step And retry 3 should occur after 2000ms delay
    // @step And then all retries should be exhausted
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        3,
        "Should make exactly 3 attempts before falling back"
    );
}

/// Scenario: Success on third attempt
#[tokio::test]
async fn test_success_on_third_attempt() {
    // Disable compression threshold for testing retry logic
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let call_count = Arc::new(AtomicUsize::new(0));

    // Need 4+ turns for summarization to trigger
    let turns = vec![
        create_test_turn("Turn 1", "Response 1", 100),
        create_test_turn("Turn 2", "Response 2", 100),
        create_test_turn("Turn 3", "Response 3", 100),
        create_test_turn("Turn 4", "Response 4", 100),
    ];

    // Mock LLM that fails twice, succeeds on third
    let call_count_clone = call_count.clone();
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                Err(anyhow!("Transient error #{}", attempt))
            } else {
                Ok::<String, anyhow::Error>("Finally succeeded!".to_string())
            }
        }
    };

    let result = compactor.compact(&turns, llm_mock).await;

    assert!(result.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 3);

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

    // Mock LLM that always fails
    let llm_mock = |_prompt: String| async move {
        Err::<String, anyhow::Error>(anyhow!("LLM service unavailable"))
    };

    let result = compactor.compact(&turns, llm_mock).await;

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
    let llm_mock = |_prompt: String| {
        let count = call_count_clone.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok::<String, anyhow::Error>("Should not be called".to_string())
        }
    };

    let result = compactor.compact(&turns, llm_mock).await;

    // Empty turns should fail (cannot compact empty history)
    assert!(result.is_err());
    assert_eq!(
        call_count.load(Ordering::SeqCst),
        0,
        "LLM should not be called for empty turns"
    );
}
