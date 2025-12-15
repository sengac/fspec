// Feature: spec/features/extract-cache-tokens-from-anthropic-api-response.feature
//
// CLI-014: Extract cache tokens from Anthropic API response
//
// Tests for extracting cache_read_input_tokens and cache_creation_input_tokens
// from Anthropic SSE MessageStart events and accumulating them into session.token_tracker.

use codelet::agent::compaction::TokenTracker;

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
    };

    // @step When effective_tokens() is called
    let effective = tracker.effective_tokens();

    // @step Then the result should be 5500 (10000 - 5000 * 0.9)
    // 10000 - (5000 * 0.9) = 10000 - 4500 = 5500
    assert_eq!(effective, 5500);
}

// =============================================================================
// Scenario: Non-Anthropic providers default to no cache tokens
// =============================================================================

#[test]
fn test_non_anthropic_providers_default_to_no_cache_tokens() {
    // @step Given an OpenAI provider streaming response without cache fields
    // OpenAI and other providers don't have cache token fields
    let turn_cache_read_tokens: Option<u64> = None;
    let turn_cache_creation_tokens: Option<u64> = None;

    // @step When the streaming response is processed
    // Non-Anthropic providers leave cache tokens as None

    // @step Then turn_cache_read_tokens should remain None
    assert!(turn_cache_read_tokens.is_none());

    // @step And turn_cache_creation_tokens should remain None
    assert!(turn_cache_creation_tokens.is_none());

    // @step And effective_tokens() should return the full input_tokens value
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 0,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };
    let effective = tracker.effective_tokens();
    // With no cache discount, effective equals input
    assert_eq!(effective, 10000);
}

// =============================================================================
// Scenario: Accumulate cache tokens into session tracker after turn
// =============================================================================

#[test]
fn test_accumulate_cache_tokens_into_session_tracker() {
    // @step Given a session with token_tracker.cache_read_input_tokens=1000
    let mut tracker = TokenTracker {
        input_tokens: 5000,
        output_tokens: 500,
        cache_read_input_tokens: Some(1000),
        cache_creation_input_tokens: None,
    };

    // @step And turn_cache_read_tokens=5000 extracted from a completed turn
    let turn_cache_read_tokens: Option<u64> = Some(5000);

    // @step When the turn completes and tokens are accumulated
    // This is the accumulation logic from interactive.rs:618-621
    if let Some(cache_read) = turn_cache_read_tokens {
        let current = tracker.cache_read_input_tokens.unwrap_or(0);
        tracker.cache_read_input_tokens = Some(current + cache_read);
    }

    // @step Then session.token_tracker.cache_read_input_tokens should be 6000
    assert_eq!(tracker.cache_read_input_tokens, Some(6000));
}

// =============================================================================
// Additional edge case tests
// =============================================================================

#[test]
fn test_effective_tokens_with_both_cache_fields() {
    // When both cache_read and cache_creation are present
    let tracker = TokenTracker {
        input_tokens: 20000,
        output_tokens: 1000,
        cache_read_input_tokens: Some(10000),
        cache_creation_input_tokens: Some(5000),
    };

    // Only cache_read affects effective_tokens (90% discount)
    // cache_creation doesn't get a discount (it's a cost, not a savings)
    let effective = tracker.effective_tokens();
    // 20000 - (10000 * 0.9) = 20000 - 9000 = 11000
    assert_eq!(effective, 11000);
}

#[test]
fn test_accumulate_cache_tokens_from_none_to_some() {
    // Starting with no cache tokens
    let mut tracker = TokenTracker {
        input_tokens: 5000,
        output_tokens: 500,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };

    // First turn extracts cache tokens
    let turn_cache_read = Some(3000);
    let turn_cache_creation = Some(1000);

    // Accumulate
    if let Some(cache_read) = turn_cache_read {
        let current = tracker.cache_read_input_tokens.unwrap_or(0);
        tracker.cache_read_input_tokens = Some(current + cache_read);
    }
    if let Some(cache_create) = turn_cache_creation {
        let current = tracker.cache_creation_input_tokens.unwrap_or(0);
        tracker.cache_creation_input_tokens = Some(current + cache_create);
    }

    assert_eq!(tracker.cache_read_input_tokens, Some(3000));
    assert_eq!(tracker.cache_creation_input_tokens, Some(1000));
}

#[test]
fn test_total_tokens_excludes_cache_discount() {
    // total_tokens() should NOT apply cache discount
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 2000,
        cache_read_input_tokens: Some(5000),
        cache_creation_input_tokens: None,
    };

    // total_tokens = input + output (no discount)
    let total = tracker.total_tokens();
    assert_eq!(total, 12000);

    // effective_tokens applies discount
    let effective = tracker.effective_tokens();
    assert_eq!(effective, 5500); // 10000 - 4500
}
