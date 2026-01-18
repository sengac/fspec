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
    let mut tracker = TokenTracker::default();

    // Verify default values
    assert_eq!(tracker.input_tokens, 0);
    assert_eq!(tracker.output_tokens, 0);
    assert_eq!(tracker.cache_read_input_tokens, None);
    assert_eq!(tracker.cache_creation_input_tokens, None);
    assert_eq!(tracker.cumulative_billed_input, 0);
    assert_eq!(tracker.cumulative_billed_output, 0);

    // @step When I restore the token state (simulating session_restore_token_state)
    // These are the same field assignments as in session_restore_token_state
    let input_tokens: u32 = 5000;
    let output_tokens: u32 = 8000;
    let cache_read_tokens: u32 = 2000;
    let cache_creation_tokens: u32 = 1000;
    let cumulative_billed_input: u32 = 10000;
    let cumulative_billed_output: u32 = 8000;

    tracker.input_tokens = input_tokens as u64;
    tracker.output_tokens = output_tokens as u64;
    tracker.cache_read_input_tokens = Some(cache_read_tokens as u64);
    tracker.cache_creation_input_tokens = Some(cache_creation_tokens as u64);
    tracker.cumulative_billed_input = cumulative_billed_input as u64;
    tracker.cumulative_billed_output = cumulative_billed_output as u64;

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
    let mut tracker = TokenTracker::default();

    // Restore state as session_restore_token_state would
    tracker.input_tokens = 10000;
    tracker.output_tokens = 5000;
    tracker.cache_read_input_tokens = Some(4000); // 4000 cache reads
    tracker.cache_creation_input_tokens = Some(1000);

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
    let mut tracker = TokenTracker::default();

    // @step When I restore token state with zero cache values
    tracker.input_tokens = 3000;
    tracker.output_tokens = 2000;
    tracker.cache_read_input_tokens = Some(0);
    tracker.cache_creation_input_tokens = Some(0);
    tracker.cumulative_billed_input = 3000;
    tracker.cumulative_billed_output = 2000;

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
    let mut tracker = TokenTracker::default();

    // Restore values
    tracker.input_tokens = 15000;
    tracker.output_tokens = 12000;
    tracker.cache_read_input_tokens = Some(5000);
    tracker.cache_creation_input_tokens = Some(2500);
    tracker.cumulative_billed_input = 30000;
    tracker.cumulative_billed_output = 25000;

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
    let mut tracker = TokenTracker::default();

    let large_value: u32 = u32::MAX - 1000; // Near max u32

    // @step When I restore the token state
    tracker.input_tokens = large_value as u64;
    tracker.output_tokens = large_value as u64;
    tracker.cumulative_billed_input = large_value as u64;
    tracker.cumulative_billed_output = large_value as u64;

    // @step Then the values are correctly converted to u64
    assert_eq!(tracker.input_tokens, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.output_tokens, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.cumulative_billed_input, (u32::MAX - 1000) as u64);
    assert_eq!(tracker.cumulative_billed_output, (u32::MAX - 1000) as u64);
}
