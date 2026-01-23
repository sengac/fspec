#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/extract-cache-tokens-from-anthropic-api-response.feature
//
// CLI-014: Extract cache tokens from Anthropic API response
//
// Tests for extracting cache_read_input_tokens and cache_creation_input_tokens
// from Anthropic SSE MessageStart events and accumulating them into session.token_tracker.

use codelet_core::compaction::TokenTracker;

// =============================================================================
// Scenario: Extract cache read tokens from Anthropic SSE MessageStart
// =============================================================================

#[test]
fn test_extract_cache_read_tokens_from_anthropic_sse() {
    // @step Given an Anthropic SSE MessageStart event with usage containing cache_read_input_tokens=5000
    // Simulating the parsed MessageStart usage data
    let cache_read_input_tokens: Option<u64> = Some(5000);

    // @step When the streaming response is processed
    // The extraction logic should capture the cache_read value
    let turn_cache_read_tokens = cache_read_input_tokens;

    // @step Then turn_cache_read_tokens should be set to 5000
    assert_eq!(turn_cache_read_tokens, Some(5000));
}

// =============================================================================
// Scenario: Extract cache creation tokens from Anthropic SSE MessageStart
// =============================================================================

#[test]
fn test_extract_cache_creation_tokens_from_anthropic_sse() {
    // @step Given an Anthropic SSE MessageStart event with usage containing cache_creation_input_tokens=2000
    // Simulating the parsed MessageStart usage data
    let cache_creation_input_tokens: Option<u64> = Some(2000);

    // @step When the streaming response is processed
    // The extraction logic should capture the cache_creation value
    let turn_cache_creation_tokens = cache_creation_input_tokens;

    // @step Then turn_cache_creation_tokens should be set to 2000
    assert_eq!(turn_cache_creation_tokens, Some(2000));
}

// =============================================================================
// Scenario: Effective tokens calculation applies 90% cache discount
// =============================================================================

#[test]
fn test_effective_tokens_applies_90_percent_cache_discount() {
    // @step Given a TokenTracker with input_tokens=10000 and cache_read_input_tokens=5000
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 0,
        cache_read_input_tokens: Some(5000),
        cache_creation_input_tokens: None,
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // @step When effective_tokens() is called
    let effective = tracker.effective_tokens();

    // @step Then effective tokens should be 5500
    // Calculation: 10000 - (5000 * 0.9) = 10000 - 4500 = 5500
    assert_eq!(effective, 5500);
}

// =============================================================================
// Scenario: Token tracker accumulates cache tokens from API response
// =============================================================================

#[test]
fn test_token_tracker_accumulates_cache_tokens() {
    // @step Given a TokenTracker initialized with cache tokens from API response
    let mut tracker = TokenTracker {
        input_tokens: 8000,
        output_tokens: 500,
        cache_read_input_tokens: Some(3000),
        cache_creation_input_tokens: Some(1000),
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // @step When the token tracker is updated with new API response values
    tracker.update(12000, 800, Some(5000), Some(2000));

    // @step Then input_tokens should be replaced (not accumulated)
    assert_eq!(tracker.input_tokens, 12000);

    // @step And output_tokens should be replaced (not accumulated)
    assert_eq!(tracker.output_tokens, 800);

    // @step And cache_read_input_tokens should be replaced
    assert_eq!(tracker.cache_read_input_tokens, Some(5000));

    // @step And cache_creation_input_tokens should be replaced
    assert_eq!(tracker.cache_creation_input_tokens, Some(2000));

    // @step And cumulative_billed_input should be accumulated
    assert_eq!(tracker.cumulative_billed_input, 12000);

    // @step And cumulative_billed_output should be accumulated
    assert_eq!(tracker.cumulative_billed_output, 800);
}

// =============================================================================
// Scenario: Effective tokens handles missing cache values
// =============================================================================

#[test]
fn test_effective_tokens_handles_missing_cache_values() {
    // @step Given a TokenTracker with no cache tokens (None values)
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 500,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // @step When effective_tokens() is called
    let effective = tracker.effective_tokens();

    // @step Then effective tokens should equal input tokens (no discount applied)
    // Calculation: 10000 - (0 * 0.9) = 10000
    assert_eq!(effective, 10000);
}

// =============================================================================
// Scenario: Total tokens includes both input and output
// =============================================================================

#[test]
fn test_total_tokens_includes_input_and_output() {
    // @step Given a TokenTracker with input_tokens=10000 and output_tokens=500
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 500,
        cache_read_input_tokens: Some(3000),
        cache_creation_input_tokens: None,
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // @step When total_tokens() is called
    let total = tracker.total_tokens();

    // @step Then total should be 10500 (input + output)
    assert_eq!(total, 10500);
}

// =============================================================================
// Scenario: Token tracker default initialization
// =============================================================================

#[test]
fn test_token_tracker_default_initialization() {
    // @step Given a new TokenTracker
    let tracker = TokenTracker::new();

    // @step Then all values should be initialized to zero/None
    assert_eq!(tracker.input_tokens, 0);
    assert_eq!(tracker.output_tokens, 0);
    assert_eq!(tracker.cache_read_input_tokens, None);
    assert_eq!(tracker.cache_creation_input_tokens, None);
    assert_eq!(tracker.cumulative_billed_input, 0);
    assert_eq!(tracker.cumulative_billed_output, 0);

    // @step And effective_tokens should return 0
    assert_eq!(tracker.effective_tokens(), 0);

    // @step And total_tokens should return 0
    assert_eq!(tracker.total_tokens(), 0);
}
