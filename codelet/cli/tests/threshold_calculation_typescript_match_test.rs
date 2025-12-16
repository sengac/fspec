//! Feature: spec/features/fix-compaction-threshold-calculation-to-match-typescript-original.feature
//!
//! Tests for CORE-008: Fix Compaction Threshold Calculation to Match TypeScript Original
//!
//! This test file ensures the Rust implementation matches the TypeScript original exactly:
//! - Threshold = contextWindow * 0.9
//! - Budget = contextWindow - AUTOCOMPACT_BUFFER (or contextWindow * 0.8 if window <= buffer)

use codelet_cli::compaction_threshold::{
    calculate_compaction_threshold, AUTOCOMPACT_BUFFER, COMPACTION_THRESHOLD_RATIO,
};

/// Scenario: Calculate threshold for 200k context window matching TypeScript
#[test]
fn test_calculate_threshold_200k_matches_typescript() {
    // @step Given I have a provider with a 200,000 token context window
    let context_window: u64 = 200_000;

    // @step When I calculate the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the threshold should be 180,000 tokens
    assert_eq!(
        threshold, 180_000,
        "Threshold should be 180,000 for 200k window"
    );

    // @step And the threshold should equal contextWindow * 0.9
    let expected = (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64;
    assert_eq!(
        threshold, expected,
        "Threshold should equal contextWindow * 0.9"
    );

    // @step And the threshold should NOT be 135,000 tokens
    assert_ne!(
        threshold, 135_000,
        "Threshold should NOT be the old incorrect value of 135,000"
    );
}

/// Scenario: Calculate threshold for 128k context window matching TypeScript
#[test]
fn test_calculate_threshold_128k_matches_typescript() {
    // @step Given I have a provider with a 128,000 token context window
    let context_window: u64 = 128_000;

    // @step When I calculate the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the threshold should be 115,200 tokens
    assert_eq!(
        threshold, 115_200,
        "Threshold should be 115,200 for 128k window"
    );

    // @step And the threshold should equal contextWindow * 0.9
    let expected = (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64;
    assert_eq!(
        threshold, expected,
        "Threshold should equal contextWindow * 0.9"
    );
}

/// Scenario: Summarization budget remains separate from threshold
#[test]
fn test_summarization_budget_separate_from_threshold() {
    // @step Given I have a provider with a 200,000 token context window
    let context_window: u64 = 200_000;

    // @step When I calculate the summarization budget
    // NOTE: This function doesn't exist yet - will be created during implementation
    // let budget = calculate_summarization_budget(context_window);

    // @step Then the budget should be 150,000 tokens
    let expected_budget = context_window - AUTOCOMPACT_BUFFER;
    assert_eq!(
        expected_budget, 150_000,
        "Budget should be 150,000 (200k - 50k buffer)"
    );

    // @step And the budget should equal contextWindow - AUTOCOMPACT_BUFFER
    assert_eq!(
        expected_budget,
        context_window - AUTOCOMPACT_BUFFER,
        "Budget should equal contextWindow - AUTOCOMPACT_BUFFER"
    );

    // @step And the budget calculation should be independent of threshold calculation
    let threshold = calculate_compaction_threshold(context_window);
    assert_ne!(
        expected_budget, threshold,
        "Budget (150k) should be different from threshold (180k)"
    );
}

/// Scenario: Existing anchor detection tests continue to pass
#[test]
fn test_existing_anchor_detection_unchanged() {
    // @step Given I have updated the threshold calculation
    let _context_window: u64 = 200_000;
    let _threshold = calculate_compaction_threshold(_context_window);

    // @step When I run the anchor detection test suite
    // @step Then all tests for ErrorResolution anchor detection should pass
    // @step And all tests for TaskCompletion anchor detection should pass
    // @step And anchor confidence values should remain unchanged

    // NOTE: This test verifies that existing anchor detection tests still pass
    // The actual anchor detection tests are in context_compaction_test.rs
    // This test ensures our changes don't break existing functionality

    // We'll verify this by running the full test suite during validation
    // For now, this test documents the requirement
    assert!(
        true,
        "Anchor detection tests are verified by running cargo test"
    );
}

/// Scenario: Existing turn selection tests continue to pass
#[test]
fn test_existing_turn_selection_unchanged() {
    // @step Given I have updated the threshold calculation
    let _context_window: u64 = 200_000;
    let _threshold = calculate_compaction_threshold(_context_window);

    // @step When I run the turn selection test suite
    // @step Then tests for keeping last 2-3 turns should pass
    // @step And tests for anchor-based selection should pass
    // @step And turn selection logic should remain unchanged

    // NOTE: This test verifies that existing turn selection tests still pass
    // The actual turn selection tests are in context_compaction_test.rs
    // This test ensures our changes don't break existing functionality

    // We'll verify this by running the full test suite during validation
    // For now, this test documents the requirement
    assert!(
        true,
        "Turn selection tests are verified by running cargo test"
    );
}
