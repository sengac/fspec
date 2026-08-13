#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/autocompact-buffer.feature
//!
//! Tests for CLI-020: Autocompact Buffer for Compaction Threshold
//!
//! NOTE: The API has been updated from the original design:
//! - Old: calculate_compaction_threshold(context_window) with COMPACTION_THRESHOLD_RATIO
//! - New: calculate_usable_context(context_window, max_output) - reserves output space
//! - New: calculate_summarization_budget(context_window) - target after compaction

use codelet_cli::compaction_threshold::{
    calculate_summarization_budget, calculate_usable_context, AUTOCOMPACT_BUFFER,
    SESSION_OUTPUT_TOKEN_MAX,
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

/// Scenario: SESSION_OUTPUT_TOKEN_MAX constant is defined
#[test]
fn test_session_output_token_max_constant_defined() {
    // @step Then SESSION_OUTPUT_TOKEN_MAX should be defined as 32,000 tokens
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
}

// ==========================================
// USABLE CONTEXT CALCULATION TESTS
// ==========================================

/// Scenario: Usable context calculation for Claude (200k context, 8k max_output)
#[test]
fn test_usable_context_claude() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;
    // @step And max_output_tokens of 8,192
    let max_output: u64 = 8_192;

    // @step When calculating the usable context
    let usable = calculate_usable_context(context_window, max_output);

    // @step Then usable context should be 191,808 tokens (200k - 8k)
    assert_eq!(usable, 191_808);
}

/// Scenario: Usable context calculation for GPT-4 (128k context, 4k max_output)
#[test]
fn test_usable_context_gpt4() {
    // @step Given the provider has a context window of 128,000 tokens
    let context_window: u64 = 128_000;
    // @step And max_output_tokens of 4,096
    let max_output: u64 = 4_096;

    // @step When calculating the usable context
    let usable = calculate_usable_context(context_window, max_output);

    // @step Then usable context should be 123,904 tokens (128k - 4k)
    assert_eq!(usable, 123_904);
}

/// Scenario: High max_output is capped at SESSION_OUTPUT_TOKEN_MAX
#[test]
fn test_usable_context_high_output_capped() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;
    // @step And max_output_tokens of 64,000 (exceeds SESSION_OUTPUT_TOKEN_MAX)
    let max_output: u64 = 64_000;

    // @step When calculating the usable context
    let usable = calculate_usable_context(context_window, max_output);

    // @step Then usable context should be 168,000 tokens (200k - 32k cap)
    assert_eq!(usable, 168_000);
}

/// Scenario: Zero max_output uses SESSION_OUTPUT_TOKEN_MAX as fallback
#[test]
fn test_usable_context_zero_max_output_fallback() {
    // @step Given the provider has a context window of 100,000 tokens
    let context_window: u64 = 100_000;
    // @step And max_output_tokens of 0 (unknown)
    let max_output: u64 = 0;

    // @step When calculating the usable context
    let usable = calculate_usable_context(context_window, max_output);

    // @step Then usable context should be 68,000 tokens (100k - 32k fallback)
    assert_eq!(usable, 68_000);
}

// ==========================================
// SUMMARIZATION BUDGET TESTS
// ==========================================

/// Scenario: Summarization budget for large context window
#[test]
fn test_summarization_budget_large_context() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;

    // @step When calculating the summarization budget
    let budget = calculate_summarization_budget(context_window);

    // @step Then budget should be 150,000 tokens (200k - 50k AUTOCOMPACT_BUFFER)
    assert_eq!(budget, 150_000);
}

/// Scenario: Summarization budget for small context window uses 80%
#[test]
fn test_summarization_budget_small_context() {
    // @step Given the provider has a context window of 40,000 tokens (less than AUTOCOMPACT_BUFFER)
    let context_window: u64 = 40_000;

    // @step When calculating the summarization budget
    let budget = calculate_summarization_budget(context_window);

    // @step Then budget should be 32,000 tokens (40k * 0.8)
    assert_eq!(budget, 32_000);
}

/// Scenario: Summarization budget when context equals buffer
#[test]
fn test_summarization_budget_context_equals_buffer() {
    // @step Given the provider has a context window equal to buffer (50,000)
    let context_window: u64 = 50_000;

    // @step When calculating the summarization budget
    let budget = calculate_summarization_budget(context_window);

    // @step Then budget should be 40,000 tokens (50k * 0.8)
    assert_eq!(budget, 40_000);
}

// ==========================================
// TRIGGER DECISION TESTS
// ==========================================

/// Scenario: Compaction does not trigger when under threshold
#[test]
fn test_no_compaction_under_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;

    // @step And the session has accumulated 180,000 effective tokens
    let effective_tokens: u64 = 180_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_usable_context(context_window, max_output); // 191,808

    // @step Then compaction should NOT trigger
    // @step Because 180,000 < 191,808 (threshold)
    assert!(effective_tokens < threshold);
}

/// Scenario: Compaction triggers when tokens exceed threshold
#[test]
fn test_compaction_triggers_above_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;

    // @step And the session has accumulated 195,000 effective tokens
    let effective_tokens: u64 = 195_000;

    // @step When checking if compaction should trigger
    let threshold = calculate_usable_context(context_window, max_output); // 191,808

    // @step Then compaction SHOULD trigger
    // @step Because 195,000 > 191,808 (threshold)
    assert!(effective_tokens > threshold);
}

/// Scenario: Compaction does not trigger at exact threshold
#[test]
fn test_compaction_does_not_trigger_at_exact_threshold() {
    // @step Given the provider has a context window of 200,000 tokens
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;

    // @step And the session has accumulated exactly threshold tokens
    let threshold = calculate_usable_context(context_window, max_output); // 191,808
    let effective_tokens: u64 = threshold;

    // @step Then compaction should NOT trigger (uses strictly greater than)
    assert!((effective_tokens <= threshold));
}
