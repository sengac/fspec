
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: Token Tracker Update Methods (CMPCT-001)
//!
//! Tests for the consolidated token tracker update methods that reduce
//! code duplication in stream_loop.rs.
//!
//! These tests verify that the new update_from_usage, update_display_only,
//! and reset_after_compaction methods behave correctly and match the
//! original inline update patterns.

use codelet_core::ApiTokenUsage;
use codelet_core::compaction::TokenTracker;

// =============================================================================
// Scenario: update_from_usage updates all fields correctly
// =============================================================================

#[test]
fn test_update_from_usage_sets_input_from_total() {
    // @step Given a token tracker with initial values
    let mut tracker = TokenTracker::default();
    
    // @step And an ApiTokenUsage with input=100k, cache_read=50k, cache_creation=5k, output=10k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    let cumulative_output = 25_000;
    
    // @step When I call update_from_usage
    tracker.update_from_usage(&usage, cumulative_output);
    
    // @step Then input_tokens should be total_input (100k + 50k + 5k = 155k)
    assert_eq!(tracker.input_tokens, 155_000);
}

#[test]
fn test_update_from_usage_sets_cumulative_output() {
    // @step Given a token tracker with initial values
    let mut tracker = TokenTracker::default();
    
    // @step And an ApiTokenUsage and cumulative_output of 25k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    let cumulative_output = 25_000;
    
    // @step When I call update_from_usage
    tracker.update_from_usage(&usage, cumulative_output);
    
    // @step Then output_tokens should be the cumulative value (25k), not the per-request value
    assert_eq!(tracker.output_tokens, 25_000);
}

#[test]
fn test_update_from_usage_accumulates_billing() {
    // @step Given a token tracker with existing billing values
    let mut tracker = TokenTracker {
        cumulative_billed_input: 50_000,
        cumulative_billed_output: 5_000,
        ..Default::default()
    };
    
    // @step And an ApiTokenUsage with input=100k, output=10k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    
    // @step When I call update_from_usage
    tracker.update_from_usage(&usage, 0);
    
    // @step Then cumulative_billed_input should accumulate (50k + 100k = 150k)
    assert_eq!(tracker.cumulative_billed_input, 150_000);
    
    // @step And cumulative_billed_output should accumulate (5k + 10k = 15k)
    assert_eq!(tracker.cumulative_billed_output, 15_000);
}

#[test]
fn test_update_from_usage_sets_cache_tokens() {
    // @step Given a token tracker
    let mut tracker = TokenTracker::default();
    
    // @step And an ApiTokenUsage with cache_read=50k, cache_creation=5k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    
    // @step When I call update_from_usage
    tracker.update_from_usage(&usage, 0);
    
    // @step Then cache_read_input_tokens should be Some(50k)
    assert_eq!(tracker.cache_read_input_tokens, Some(50_000));
    
    // @step And cache_creation_input_tokens should be Some(5k)
    assert_eq!(tracker.cache_creation_input_tokens, Some(5_000));
}

// =============================================================================
// Scenario: update_display_only does NOT accumulate billing
// =============================================================================

#[test]
fn test_update_display_only_does_not_accumulate_billing() {
    // @step Given a token tracker with existing billing values
    let mut tracker = TokenTracker {
        cumulative_billed_input: 50_000,
        cumulative_billed_output: 5_000,
        ..Default::default()
    };
    
    // @step And an ApiTokenUsage with input=100k, output=10k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    
    // @step When I call update_display_only
    tracker.update_display_only(&usage, 25_000);
    
    // @step Then cumulative_billed_input should NOT change (still 50k)
    assert_eq!(tracker.cumulative_billed_input, 50_000);
    
    // @step And cumulative_billed_output should NOT change (still 5k)
    assert_eq!(tracker.cumulative_billed_output, 5_000);
}

#[test]
fn test_update_display_only_sets_display_values() {
    // @step Given a token tracker
    let mut tracker = TokenTracker::default();
    
    // @step And an ApiTokenUsage with total_input = 155k
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    let cumulative_output = 25_000;
    
    // @step When I call update_display_only
    tracker.update_display_only(&usage, cumulative_output);
    
    // @step Then input_tokens should be set to total_input (155k)
    assert_eq!(tracker.input_tokens, 155_000);
    
    // @step And output_tokens should be set to cumulative_output (25k)
    assert_eq!(tracker.output_tokens, 25_000);
    
    // @step And cache tokens should be set
    assert_eq!(tracker.cache_read_input_tokens, Some(50_000));
    assert_eq!(tracker.cache_creation_input_tokens, Some(5_000));
}

// =============================================================================
// Scenario: reset_after_compaction clears output and cache but preserves billing
// =============================================================================

#[test]
fn test_reset_after_compaction_clears_output_and_cache() {
    // @step Given a token tracker with all fields populated
    let mut tracker = TokenTracker {
        input_tokens: 100_000,
        output_tokens: 25_000,
        cumulative_billed_input: 200_000,
        cumulative_billed_output: 50_000,
        cache_read_input_tokens: Some(50_000),
        cache_creation_input_tokens: Some(5_000),
    };
    
    // @step When I call reset_after_compaction
    tracker.reset_after_compaction();
    
    // @step Then output_tokens should be 0
    assert_eq!(tracker.output_tokens, 0);
    
    // @step And cache_read_input_tokens should be None
    assert_eq!(tracker.cache_read_input_tokens, None);
    
    // @step And cache_creation_input_tokens should be None
    assert_eq!(tracker.cache_creation_input_tokens, None);
}

#[test]
fn test_reset_after_compaction_preserves_billing() {
    // @step Given a token tracker with billing values
    let mut tracker = TokenTracker {
        cumulative_billed_input: 200_000,
        cumulative_billed_output: 50_000,
        ..Default::default()
    };
    
    // @step When I call reset_after_compaction
    tracker.reset_after_compaction();
    
    // @step Then cumulative_billed_input should be preserved (200k)
    assert_eq!(tracker.cumulative_billed_input, 200_000);
    
    // @step And cumulative_billed_output should be preserved (50k)
    assert_eq!(tracker.cumulative_billed_output, 50_000);
}

#[test]
fn test_reset_after_compaction_preserves_input_tokens() {
    // @step Given a token tracker with input_tokens set
    let mut tracker = TokenTracker {
        input_tokens: 100_000,
        ..Default::default()
    };
    
    // @step When I call reset_after_compaction
    tracker.reset_after_compaction();
    
    // @step Then input_tokens should be preserved (set by execute_compaction)
    assert_eq!(tracker.input_tokens, 100_000);
}

// =============================================================================
// Scenario: Multiple updates accumulate billing correctly
// =============================================================================

#[test]
fn test_multiple_update_from_usage_accumulates_billing() {
    // @step Given a fresh token tracker
    let mut tracker = TokenTracker::default();
    
    // @step When I call update_from_usage three times with different values
    let usage1 = ApiTokenUsage::new(100_000, 0, 0, 10_000);
    tracker.update_from_usage(&usage1, 10_000);
    
    let usage2 = ApiTokenUsage::new(105_000, 50_000, 0, 8_000);
    tracker.update_from_usage(&usage2, 18_000);
    
    let usage3 = ApiTokenUsage::new(110_000, 60_000, 5_000, 12_000);
    tracker.update_from_usage(&usage3, 30_000);
    
    // @step Then cumulative_billed_input should be sum of all input_tokens (100k + 105k + 110k = 315k)
    assert_eq!(tracker.cumulative_billed_input, 315_000);
    
    // @step And cumulative_billed_output should be sum of all output_tokens (10k + 8k + 12k = 30k)
    assert_eq!(tracker.cumulative_billed_output, 30_000);
    
    // @step And input_tokens should be the LAST total_input (110k + 60k + 5k = 175k)
    assert_eq!(tracker.input_tokens, 175_000);
    
    // @step And output_tokens should be the LAST cumulative_output (30k)
    assert_eq!(tracker.output_tokens, 30_000);
}

// =============================================================================
// Scenario: Equivalence with original inline pattern
// =============================================================================

#[test]
fn test_update_from_usage_matches_original_pattern() {
    // @step Given two token trackers
    let mut tracker_new = TokenTracker {
        cumulative_billed_input: 50_000,
        cumulative_billed_output: 5_000,
        ..Default::default()
    };
    let mut tracker_old = tracker_new.clone();
    
    // @step And the same usage values
    let usage = ApiTokenUsage::new(100_000, 50_000, 5_000, 10_000);
    let cumulative_output = 25_000;
    
    // @step When I use the new update_from_usage method
    tracker_new.update_from_usage(&usage, cumulative_output);
    
    // @step And I use the original inline pattern
    tracker_old.input_tokens = usage.total_input();
    tracker_old.output_tokens = cumulative_output;
    tracker_old.cumulative_billed_input += usage.input_tokens;
    tracker_old.cumulative_billed_output += usage.output_tokens;
    tracker_old.cache_read_input_tokens = Some(usage.cache_read_input_tokens);
    tracker_old.cache_creation_input_tokens = Some(usage.cache_creation_input_tokens);
    
    // @step Then both trackers should have identical state
    assert_eq!(tracker_new.input_tokens, tracker_old.input_tokens);
    assert_eq!(tracker_new.output_tokens, tracker_old.output_tokens);
    assert_eq!(tracker_new.cumulative_billed_input, tracker_old.cumulative_billed_input);
    assert_eq!(tracker_new.cumulative_billed_output, tracker_old.cumulative_billed_output);
    assert_eq!(tracker_new.cache_read_input_tokens, tracker_old.cache_read_input_tokens);
    assert_eq!(tracker_new.cache_creation_input_tokens, tracker_old.cache_creation_input_tokens);
}
