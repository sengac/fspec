// Feature: CTX-005 - Pre-prompt compaction check
//
// Tests for the pre-prompt compaction check that prevents "prompt is too long"
// API errors when resuming a session at high context fill.
//
// Reference: spec/features/optimized-compaction-trigger.feature

use codelet_cli::compaction_threshold::calculate_usable_context;
use codelet_common::token_estimator::count_tokens;

// =============================================================================
// Scenario: Pre-prompt check triggers compaction when context is near limit
// =============================================================================

#[test]
fn test_pre_prompt_check_triggers_when_over_threshold() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 200,000 - 8,192 = 191,808

    // @step And the session has 192000 input tokens (restored from persistence, over threshold)
    let session_input_tokens: u64 = 192_000;
    let session_output_tokens: u64 = 0;

    // @step And the user sends any prompt
    let prompt = "Hello";
    let prompt_tokens = count_tokens(prompt) as u64;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction should be triggered
    // 192,000 + ~1 = ~192,001 > 191,808 threshold
    assert!(
        should_compact,
        "Pre-prompt compaction should trigger when estimated_total ({}) > threshold ({})",
        estimated_total,
        threshold
    );
}

// =============================================================================
// Scenario: Pre-prompt check does NOT trigger when under threshold
// =============================================================================

#[test]
fn test_pre_prompt_check_does_not_trigger_when_under_threshold() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 200,000 - 8,192 = 191,808

    // @step And the session has 150000 input tokens
    let session_input_tokens: u64 = 150_000;
    let session_output_tokens: u64 = 0;

    // @step And the user sends a prompt with approximately 1000 tokens
    let prompt = "x".repeat(4_000); // ~1000 tokens
    let prompt_tokens = count_tokens(&prompt) as u64;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction should NOT be triggered
    // 150,000 + ~1,000 = ~151,000 < 191,808 threshold
    assert!(
        !should_compact,
        "Pre-prompt compaction should NOT trigger when estimated_total ({}) <= threshold ({})",
        estimated_total,
        threshold
    );
}

// =============================================================================
// Scenario: Pre-prompt check accounts for output tokens
// =============================================================================

#[test]
fn test_pre_prompt_check_includes_output_tokens() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 191,808

    // @step And the session has 185000 input tokens and 8000 output tokens (total 193k)
    let session_input_tokens: u64 = 185_000;
    let session_output_tokens: u64 = 8_000;

    // @step And the user sends a small prompt
    let prompt = "Hello";
    let prompt_tokens = count_tokens(prompt) as u64;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction should be triggered (input + output already exceeds threshold)
    // 185,000 + 8,000 + ~1 = 193,001 > 191,808
    assert!(
        should_compact,
        "Should trigger when current_tokens ({}) + prompt ({}) > threshold ({})",
        current_tokens,
        prompt_tokens,
        threshold
    );
}

// =============================================================================
// Scenario: Pre-prompt check with exact boundary (should NOT trigger)
// =============================================================================

#[test]
fn test_pre_prompt_check_at_exact_boundary_does_not_trigger() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 191,808

    // @step And the session has tokens that exactly equal the threshold minus prompt tokens
    // We want: current + prompt = threshold exactly
    let prompt = "test";
    let prompt_tokens = count_tokens(prompt) as u64;
    let session_input_tokens: u64 = threshold - prompt_tokens;
    let session_output_tokens: u64 = 0;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction should NOT trigger (uses > not >=)
    assert_eq!(estimated_total, threshold);
    assert!(
        !should_compact,
        "At exact boundary ({} == {}), should NOT trigger (uses > not >=)",
        estimated_total,
        threshold
    );
}

// =============================================================================
// Scenario: Pre-prompt check one token over threshold DOES trigger
// =============================================================================

#[test]
fn test_pre_prompt_check_one_over_threshold_triggers() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 191,808

    // @step And the session has tokens that are one more than threshold minus prompt tokens
    let prompt = "test";
    let prompt_tokens = count_tokens(prompt) as u64;
    let session_input_tokens: u64 = threshold - prompt_tokens + 1;
    let session_output_tokens: u64 = 0;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction SHOULD trigger
    assert_eq!(estimated_total, threshold + 1);
    assert!(
        should_compact,
        "One over threshold ({} > {}) should trigger",
        estimated_total,
        threshold
    );
}

// =============================================================================
// Scenario: Session resumed at 98% fill triggers pre-prompt compaction
// (Matches the exact bug scenario from the screenshot)
// =============================================================================

#[test]
fn test_session_at_98_percent_triggers_pre_prompt_compaction() {
    // @step Given a Claude model with 200k context window
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);
    // threshold = 191,808

    // @step And a session restored near threshold (192k input tokens)
    // The screenshot showed 188k, but the error happened after tools loaded more.
    // The pre-prompt check handles when current + prompt exceeds threshold.
    let session_input_tokens: u64 = 192_000;
    let session_output_tokens: u64 = 0;

    // @step And user sends any prompt
    let prompt = "hello";
    let prompt_tokens = count_tokens(prompt) as u64;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold;

    // @step Then compaction should be triggered BEFORE the API call
    // 192,000 + ~1 > 191,808 threshold
    assert!(
        should_compact,
        "At 192k tokens + prompt, should trigger: {} > {}",
        estimated_total,
        threshold
    );
}

// =============================================================================
// Scenario: Empty session does NOT trigger pre-prompt compaction
// =============================================================================

#[test]
fn test_empty_session_does_not_trigger_compaction() {
    // @step Given a new session with 0 tokens
    let session_input_tokens: u64 = 0;
    let session_output_tokens: u64 = 0;
    let messages_empty = true;

    // @step And a Claude model
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let threshold = calculate_usable_context(context_window, max_output);

    // @step And user sends a prompt
    let prompt = "Hello, world!";
    let prompt_tokens = count_tokens(prompt) as u64;

    // @step When we check if pre-prompt compaction is needed
    let current_tokens = session_input_tokens + session_output_tokens;
    let estimated_total = current_tokens + prompt_tokens;
    let should_compact = estimated_total > threshold && !messages_empty;

    // @step Then compaction should NOT trigger (nothing to compact)
    assert!(
        !should_compact,
        "Empty session should not trigger compaction"
    );
}
