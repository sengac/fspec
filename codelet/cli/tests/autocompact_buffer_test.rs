//! Feature: spec/features/autocompact-buffer.feature
//!
//! Tests for CLI-020: Autocompact Buffer for Compaction Threshold

use codelet_cli::compaction_threshold::{
    calculate_compaction_threshold, AUTOCOMPACT_BUFFER, COMPACTION_THRESHOLD_RATIO,
};

// ==========================================
// CONSTANT DEFINITION TESTS
// ==========================================

/// Scenario: AUTOCOMPACT_BUFFER constant is defined
#[test]
fn test_autocompact_buffer_constant_defined() {
    // @step Then AUTOCOMPACT_BUFFER should be defined as 50,000 tokens
    assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
}

/// Scenario: COMPACTION_THRESHOLD_RATIO constant is defined
#[test]
fn test_compaction_threshold_ratio_constant_defined() {
    // @step Then COMPACTION_THRESHOLD_RATIO should be defined as 0.9 (90%)
    assert!((COMPACTION_THRESHOLD_RATIO - 0.9).abs() < f64::EPSILON);
}

// ==========================================
// THRESHOLD CALCULATION TESTS
// ==========================================

/// Scenario: Compaction threshold accounts for autocompact buffer (Claude)
#[test]
fn test_compaction_threshold_with_buffer_claude() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the summarization budget should be 150,000 tokens (context_window - buffer)
    let expected_budget = context_window - AUTOCOMPACT_BUFFER;
    assert_eq!(expected_budget, 150_000);

    // @step And the threshold should be 135,000 tokens (budget * 0.9)
    let expected_threshold = (expected_budget as f64 * COMPACTION_THRESHOLD_RATIO) as u64;
    assert_eq!(expected_threshold, 135_000);
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Buffer scales appropriately for OpenAI
#[test]
fn test_compaction_threshold_with_buffer_openai() {
    // @step Given the provider has a context window of 128,000 tokens
    let context_window: u64 = 128_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be (128,000 - 50,000) * 0.9 = 70,200
    let expected_budget = 128_000 - 50_000; // 78,000
    let expected_threshold = (expected_budget as f64 * 0.9) as u64; // 70,200
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Buffer scales appropriately for Codex
#[test]
fn test_compaction_threshold_with_buffer_codex() {
    // @step Given the provider has a context window of 272,000 tokens
    let context_window: u64 = 272_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be (272,000 - 50,000) * 0.9 = 199,800
    let expected_budget = 272_000 - 50_000; // 222,000
    let expected_threshold = (expected_budget as f64 * 0.9) as u64; // 199,800
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Buffer scales appropriately for Gemini
#[test]
fn test_compaction_threshold_with_buffer_gemini() {
    // @step Given the provider has a context window of 1,000,000 tokens
    let context_window: u64 = 1_000_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be (1,000,000 - 50,000) * 0.9 = 855,000
    let expected_budget = 1_000_000 - 50_000; // 950,000
    let expected_threshold = (expected_budget as f64 * 0.9) as u64; // 855,000
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Buffer handles edge case where context window is small
#[test]
fn test_compaction_threshold_small_context_window() {
    // @step Given the provider has a context window of 60,000 tokens
    let context_window: u64 = 60_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the summarization budget should be 10,000 tokens (using saturating subtraction)
    // @step And the threshold should be 9,000 tokens (budget * 0.9)
    let expected_budget = 60_000_u64.saturating_sub(50_000); // 10,000
    let expected_threshold = (expected_budget as f64 * 0.9) as u64; // 9,000
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Buffer handles edge case where context window equals buffer
#[test]
fn test_compaction_threshold_context_equals_buffer() {
    // @step Given the provider has a context window equal to buffer
    let context_window: u64 = 50_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 0 (saturating subtraction yields 0)
    assert_eq!(threshold, 0);
}

/// Scenario: Buffer handles edge case where context window is smaller than buffer
#[test]
fn test_compaction_threshold_context_smaller_than_buffer() {
    // @step Given the provider has a context window smaller than buffer
    let context_window: u64 = 30_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 0 (saturating subtraction prevents underflow)
    assert_eq!(threshold, 0);
}

// ==========================================
// TRIGGER DECISION TESTS
// ==========================================

/// Scenario: Buffer does not cause premature compaction for small contexts
#[test]
fn test_no_premature_compaction() {
    // @step Given the provider has a context window of 128,000 tokens
    let context_window: u64 = 128_000;

    // @step And the session has accumulated 60,000 effective tokens
    let effective_tokens: u64 = 60_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window);
    let should_trigger = effective_tokens > threshold;

    // @step Then compaction should NOT trigger
    // @step Because 60,000 < 70,200 (threshold)
    assert!(!should_trigger);
    assert!(threshold > effective_tokens);
}

/// Scenario: Compaction triggers when tokens exceed threshold
#[test]
fn test_compaction_triggers_above_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step And the session has accumulated 140,000 effective tokens
    let effective_tokens: u64 = 140_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window); // 135,000

    // @step Then compaction SHOULD trigger
    // @step Because 140,000 > 135,000 (threshold)
    assert!(effective_tokens > threshold);
}

/// Scenario: Compaction does not trigger just below threshold
#[test]
fn test_compaction_does_not_trigger_below_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step And the session has accumulated 130,000 effective tokens
    let effective_tokens: u64 = 130_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window); // 135,000

    // @step Then compaction should NOT trigger
    // @step Because 130,000 < 135,000 (threshold)
    assert!(effective_tokens < threshold);
}
