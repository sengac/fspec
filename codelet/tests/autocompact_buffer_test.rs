//! Feature: spec/features/autocompact-buffer.feature
//!
//! Tests for CLI-020: Autocompact Buffer for Compaction Threshold

use codelet::cli::compaction_threshold::{
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

/// Scenario: Compaction threshold matches TypeScript (Claude)
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_with_buffer_claude() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the threshold should be 180,000 tokens (context_window * 0.9)
    let expected_threshold = (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64;
    assert_eq!(expected_threshold, 180_000);
    assert_eq!(threshold, expected_threshold);

    // Note: Summarization budget is now calculated separately
    // Budget would be 150,000 (context_window - buffer)
}

/// Scenario: Threshold matches TypeScript for OpenAI
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_with_buffer_openai() {
    // @step Given the provider has a context window of 128,000 tokens
    let context_window: u64 = 128_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 128,000 * 0.9 = 115,200
    let expected_threshold = (context_window as f64 * 0.9) as u64; // 115,200
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Threshold matches TypeScript for Codex
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_with_buffer_codex() {
    // @step Given the provider has a context window of 272,000 tokens
    let context_window: u64 = 272_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 272,000 * 0.9 = 244,800
    let expected_threshold = (context_window as f64 * 0.9) as u64; // 244,800
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Threshold matches TypeScript for Gemini
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_with_buffer_gemini() {
    // @step Given the provider has a context window of 1,000,000 tokens
    let context_window: u64 = 1_000_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 1,000,000 * 0.9 = 900,000
    let expected_threshold = (context_window as f64 * 0.9) as u64; // 900,000
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Threshold matches TypeScript for small context window
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_small_context_window() {
    // @step Given the provider has a context window of 60,000 tokens
    let context_window: u64 = 60_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then the threshold should be 60,000 * 0.9 = 54,000
    let expected_threshold = (context_window as f64 * 0.9) as u64; // 54,000
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Threshold matches TypeScript when context window equals buffer
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_context_equals_buffer() {
    // @step Given the provider has a context window equal to buffer
    let context_window: u64 = 50_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 50,000 * 0.9 = 45,000
    let expected_threshold = (context_window as f64 * 0.9) as u64;
    assert_eq!(threshold, expected_threshold);
}

/// Scenario: Threshold matches TypeScript for very small context window
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_threshold_context_smaller_than_buffer() {
    // @step Given the provider has a context window smaller than buffer
    let context_window: u64 = 30_000;

    // @step When calculating the compaction threshold
    let threshold = calculate_compaction_threshold(context_window);

    // @step Then threshold should be 30,000 * 0.9 = 27,000
    let expected_threshold = (context_window as f64 * 0.9) as u64;
    assert_eq!(threshold, expected_threshold);
}

// ==========================================
// TRIGGER DECISION TESTS
// ==========================================

/// Scenario: Compaction does not trigger when below threshold
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_no_premature_compaction() {
    // @step Given the provider has a context window of 128,000 tokens
    let context_window: u64 = 128_000;

    // @step And the session has accumulated 60,000 effective tokens
    let effective_tokens: u64 = 60_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window); // Now 115,200
    let should_trigger = effective_tokens > threshold;

    // @step Then compaction should NOT trigger
    // @step Because 60,000 < 115,200 (threshold)
    assert!(!should_trigger);
    assert!(threshold > effective_tokens);
}

/// Scenario: Compaction triggers when tokens exceed threshold
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_triggers_above_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step And the session has accumulated 185,000 effective tokens (above threshold)
    let effective_tokens: u64 = 185_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window); // Now 180,000

    // @step Then compaction SHOULD trigger
    // @step Because 185,000 > 180,000 (threshold)
    assert!(effective_tokens > threshold);
}

/// Scenario: Compaction does not trigger just below threshold
/// UPDATED: Now matches TypeScript implementation (threshold = contextWindow * 0.9)
#[test]
fn test_compaction_does_not_trigger_below_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step And the session has accumulated 175,000 effective tokens (below threshold)
    let effective_tokens: u64 = 175_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_compaction_threshold(context_window); // Now 180,000

    // @step Then compaction should NOT trigger
    // @step Because 175,000 < 180,000 (threshold)
    assert!(effective_tokens < threshold);
}
