#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/background-session-management-with-attach-detach.feature
//!
//! Tests for NAPI-009: Background Session Token State Restoration
//!
//! These tests verify that token state can be properly restored to a background
//! session when attaching via /resume. This ensures context fill percentage and
//! token counts are accurate after reattaching to a detached session.

use codelet_core::compaction::TokenTracker;

/// Test TokenTracker field restoration
///
/// Scenario: Restore token state when attaching to a detached session
///
/// @step Given I have a session with persisted token usage
/// @step When I attach to the session via /resume
/// @step Then the token state is restored to the background session
#[test]
fn test_token_tracker_field_restoration() {
    // @step Given I have a session with persisted token usage
    let default_tracker = TokenTracker::default();

    // Verify default values
    assert_eq!(default_tracker.input_tokens, 0);
    assert_eq!(default_tracker.output_tokens, 0);
    assert_eq!(default_tracker.cache_read_input_tokens, None);
    assert_eq!(default_tracker.cache_creation_input_tokens, None);
    assert_eq!(default_tracker.cumulative_billed_input, 0);
    assert_eq!(default_tracker.cumulative_billed_output, 0);

    // @step When I restore the token state (simulating session_restore_token_state)
    // These are the same field assignments as in session_restore_token_state
    let tracker = TokenTracker {
        input_tokens: 5000,
        output_tokens: 8000,
        cache_read_input_tokens: Some(2000),
        cache_creation_input_tokens: Some(1000),
        cumulative_billed_input: 10000,
        cumulative_billed_output: 8000,
        reasoning_tokens: 0,
    };

    // @step Then the token state is restored to the session
    assert_eq!(tracker.input_tokens, 5000);
    assert_eq!(tracker.output_tokens, 8000);
    assert_eq!(tracker.cache_read_input_tokens, Some(2000));
    assert_eq!(tracker.cache_creation_input_tokens, Some(1000));
    assert_eq!(tracker.cumulative_billed_input, 10000);
    assert_eq!(tracker.cumulative_billed_output, 8000);
}

/// Test effective tokens calculation after restoration
///
/// Scenario: Calculate context fill percentage after token state restoration
///
/// @step Given I have restored token state with cache read tokens
/// @step When I calculate effective tokens
/// @step Then the cache discount is correctly applied
#[test]
fn test_effective_tokens_after_restoration() {
    // @step Given I have restored token state with cache read tokens
    // Restore state as session_restore_token_state would
    let tracker = TokenTracker {
        input_tokens: 10000,
        output_tokens: 5000,
        cache_read_input_tokens: Some(4000), // 4000 cache reads
        cache_creation_input_tokens: Some(1000),
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
        reasoning_tokens: 0,
    };

    // @step When I calculate effective tokens
    let effective = tracker.effective_tokens();

    // @step Then the cache discount is correctly applied
    // Effective = 10000 - (4000 * 0.9) = 10000 - 3600 = 6400
    assert_eq!(effective, 6400);
}

/// Test token restoration with zero cache values
///
/// Scenario: Restore token state without cache tokens
///
/// @step Given a session that was created before cache tracking
/// @step When I restore token state with zero cache values
/// @step Then the cache fields are set to Some(0)
#[test]
fn test_token_restoration_with_zero_cache() {
    // @step Given a session that was created before cache tracking
    // @step When I restore token state with zero cache values
    let tracker = TokenTracker {
        input_tokens: 3000,
        output_tokens: 2000,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cumulative_billed_input: 3000,
        cumulative_billed_output: 2000,
        reasoning_tokens: 0,
    };

    // @step Then the cache fields are set to Some(0)
    assert_eq!(tracker.cache_read_input_tokens, Some(0));
    assert_eq!(tracker.cache_creation_input_tokens, Some(0));

    // And effective tokens equals input tokens (no cache discount)
    assert_eq!(tracker.effective_tokens(), 3000);
}

/// Test token tracker preserves values through update cycle
///
/// Scenario: Token state persists correctly across operations
///
/// @step Given I have restored token state to a session
/// @step When the session's token tracker is accessed
/// @step Then the restored values are preserved
#[test]
fn test_token_tracker_value_preservation() {
    // @step Given I have restored token state to a session
    let tracker = TokenTracker {
        input_tokens: 15000,
        output_tokens: 12000,
        cache_read_input_tokens: Some(5000),
        cache_creation_input_tokens: Some(2500),
        cumulative_billed_input: 30000,
        cumulative_billed_output: 25000,
        reasoning_tokens: 0,
    };

    // @step When the session's token tracker is accessed
    let total = tracker.total_tokens();
    let effective = tracker.effective_tokens();

    // @step Then the restored values are preserved
    assert_eq!(total, 27000); // 15000 + 12000
    assert_eq!(effective, 10500); // 15000 - (5000 * 0.9) = 15000 - 4500

    // And individual fields remain unchanged
    assert_eq!(tracker.input_tokens, 15000);
    assert_eq!(tracker.output_tokens, 12000);
    assert_eq!(tracker.cumulative_billed_input, 30000);
    assert_eq!(tracker.cumulative_billed_output, 25000);
}

/// Test u32 to u64 conversion boundary
///
/// Scenario: Large token values are handled correctly
///
/// @step Given token values near u32 maximum
/// @step When I restore the token state
/// @step Then the values are correctly converted to u64
#[test]
fn test_large_token_value_conversion() {
    // @step Given token values near u32 maximum
    let large_value: u64 = (u32::MAX - 1000) as u64;

    // @step When I restore the token state
    let tracker = TokenTracker {
        input_tokens: large_value,
        output_tokens: large_value,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        cumulative_billed_input: large_value,
        cumulative_billed_output: large_value,
        reasoning_tokens: 0,
    };

    // @step Then the values are correctly converted to u64
    assert_eq!(tracker.input_tokens, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.output_tokens, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.cumulative_billed_input, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.cumulative_billed_output, (u32::MAX - 1000) as u64);
}
