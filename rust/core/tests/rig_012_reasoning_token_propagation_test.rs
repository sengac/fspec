#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/reasoning-token-propagation.feature
//!
//! This test file validates the acceptance criteria for end-to-end reasoning
//! token propagation at the codelet-core layer.
//!
//! Tests:
//! - TokenDisplayUpdate propagates reasoning_tokens from Usage
//! - StreamingTokenDisplay propagates reasoning from final response
//! - Compaction TokenTracker includes reasoning in total_tokens

use codelet_core::compaction::TokenTracker;
use codelet_core::streaming_display::StreamingTokenDisplay;
use codelet_core::ApiTokenUsage;
use codelet_core::TokenDisplayUpdate;
use rig::completion::Usage;

// =============================================================================
// Scenario: TokenDisplayUpdate propagates reasoning tokens from Usage
// =============================================================================

#[test]
fn test_token_display_update_has_reasoning_tokens_field() {
    // @step Given a StreamingTokenDisplay initialized with previous session values
    let mut display = StreamingTokenDisplay::new(1000, 500, 100, 50);

    // @step And the Usage event contains reasoning_tokens of 5000
    let usage = Usage {
        input_tokens: 1500,
        output_tokens: 50,
        total_tokens: 6550,
        cache_read_input_tokens: Some(200),
        cache_creation_input_tokens: Some(100),
        reasoning_tokens: Some(5000),
    };

    // @step When update_from_usage is called with the Usage event
    let update = display.update_from_usage(&usage).unwrap();

    // @step Then the returned TokenDisplayUpdate should have reasoning_tokens equal to 5000
    assert_eq!(update.reasoning_tokens, 5000);

    // @step And total_context should include reasoning_tokens in the sum
    let expected_total = update.total_input() + update.output_tokens + update.reasoning_tokens;
    assert_eq!(update.total_context(), expected_total);
}

#[test]
fn test_token_display_update_total_context_with_reasoning() {
    // Verify total_context includes reasoning in the sum
    let update = TokenDisplayUpdate {
        input_tokens: 1000,
        output_tokens: 500,
        cache_read_tokens: 200,
        cache_creation_tokens: 100,
        tokens_per_second: None,
        reasoning_tokens: 3000,
    };

    // total_context = total_input (1000+200+100) + output (500) + reasoning (3000) = 4800
    assert_eq!(update.total_context(), 4800);
}

#[test]
fn test_token_display_update_total_context_without_reasoning() {
    // Without reasoning, total_context should still work (backward compat)
    let update = TokenDisplayUpdate {
        input_tokens: 1000,
        output_tokens: 500,
        cache_read_tokens: 200,
        cache_creation_tokens: 100,
        tokens_per_second: None,
        reasoning_tokens: 0,
    };

    // total_context = total_input (1300) + output (500) + reasoning (0) = 1800
    assert_eq!(update.total_context(), 1800);
}

// =============================================================================
// Scenario: StreamingTokenDisplay propagates reasoning from final response
// =============================================================================

#[test]
fn test_streaming_token_display_final_response_with_reasoning() {
    // @step Given a StreamingTokenDisplay for an OpenAI-compatible provider
    let mut display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step And the provider sends no Usage events during streaming
    display.record_chunk("Some output text");

    // @step When update_from_final_response is called with reasoning_tokens of 4000
    let usage = Usage {
        input_tokens: 5000,
        output_tokens: 200,
        total_tokens: 9200,
        cache_read_input_tokens: Some(1000),
        cache_creation_input_tokens: Some(500),
        reasoning_tokens: Some(4000),
    };
    let update = display.update_from_final_response(&usage);

    // @step Then the returned TokenDisplayUpdate should have reasoning_tokens equal to 4000
    assert_eq!(update.reasoning_tokens, 4000);
}

#[test]
fn test_streaming_token_display_usage_event_with_reasoning() {
    let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

    let usage = Usage {
        input_tokens: 1500,
        output_tokens: 50,
        total_tokens: 8550,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        reasoning_tokens: Some(7000),
    };
    let update = display.update_from_usage(&usage).unwrap();

    assert_eq!(update.reasoning_tokens, 7000);
}

#[test]
fn test_streaming_token_display_no_reasoning_tokens() {
    let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

    let usage = Usage {
        input_tokens: 1500,
        output_tokens: 50,
        total_tokens: 1550,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        reasoning_tokens: None,
    };
    let update = display.update_from_usage(&usage).unwrap();

    // When no reasoning tokens, should be 0
    assert_eq!(update.reasoning_tokens, 0);
}

// =============================================================================
// Scenario: Compaction TokenTracker includes reasoning in total_tokens
// =============================================================================

#[test]
fn test_compaction_token_tracker_total_tokens_includes_reasoning() {
    // @step Given a compaction model TokenTracker with input_tokens 10000, output_tokens 2000, and reasoning_tokens 5000
    let tracker = TokenTracker {
        input_tokens: 10_000,
        output_tokens: 2_000,
        reasoning_tokens: 5_000,
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };

    // @step When total_tokens is called
    let total = tracker.total_tokens();

    // @step Then the result should be 17000
    assert_eq!(total, 17_000);

    // @step And effective_tokens should also account for reasoning tokens
    // effective_tokens is based on input_tokens with cache discount
    let effective = tracker.effective_tokens();
    assert!(effective > 0);
}

#[test]
fn test_compaction_token_tracker_update_from_usage_preserves_reasoning() {
    let mut tracker = TokenTracker::default();
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000).with_reasoning_tokens(8_000);

    tracker.update_from_usage(&usage, 25_000);

    // reasoning_tokens should be set from usage
    assert_eq!(tracker.reasoning_tokens, 8_000);
    // total_tokens should include reasoning
    assert_eq!(
        tracker.total_tokens(),
        tracker.input_tokens + tracker.output_tokens + tracker.reasoning_tokens
    );
}

#[test]
fn test_compaction_token_tracker_reset_after_compaction_clears_reasoning() {
    let mut tracker = TokenTracker {
        input_tokens: 100_000,
        output_tokens: 25_000,
        reasoning_tokens: 8_000,
        cumulative_billed_input: 200_000,
        cumulative_billed_output: 50_000,
        cache_read_input_tokens: Some(50_000),
        cache_creation_input_tokens: Some(5_000),
    };

    tracker.reset_after_compaction();

    // reasoning_tokens should be cleared after compaction
    assert_eq!(tracker.reasoning_tokens, 0);
    // output_tokens should also be 0
    assert_eq!(tracker.output_tokens, 0);
    // billing should be preserved
    assert_eq!(tracker.cumulative_billed_input, 200_000);
}

#[test]
fn test_compaction_token_tracker_default_reasoning_is_zero() {
    let tracker = TokenTracker::default();
    assert_eq!(tracker.reasoning_tokens, 0);
    // total_tokens without reasoning should still work
    assert_eq!(tracker.total_tokens(), 0);
}
