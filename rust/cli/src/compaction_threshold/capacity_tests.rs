use super::*;
use codelet_core::{CompactionHook, TokenState};
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::message::Message;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use std::sync::{Arc, Mutex};

/// Run the production pre-request check (`on_completion_call`) for a
/// 200k/8192 Claude model's default threshold and report whether it
/// requested compaction.
async fn hook_triggers(state: TokenState) -> bool {
    let threshold = compaction_threshold_for(200_000, 8_192, Some("claude-sonnet-4"), None);
    assert_eq!(
        threshold, 160_000,
        "80% of 200k, under the 191,808 usable context"
    );
    let shared = Arc::new(Mutex::new(state));
    let hook = CompactionHook::new(Arc::clone(&shared), threshold);
    let signal = CancelSignal::new();
    <CompactionHook as StreamingPromptHook<ResponsesCompletionModel>>::on_completion_call(
        &hook,
        &Message::user("continue"),
        &[],
        signal.clone(),
    )
    .await;
    let flagged = shared.lock().is_ok_and(|current| current.compaction_needed);
    assert_eq!(
        flagged,
        signal.is_cancelled(),
        "flag and cancellation agree"
    );
    flagged
}

#[test]
fn test_constants_defined() {
    assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
}

#[test]
fn test_usable_context_claude_sonnet() {
    // @step Given a model with context_window of 200000 tokens
    let context_window = 200_000;
    // @step And the model has max_output_tokens of 8192
    let max_output = 8_192;
    // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    // @step When I calculate usable context
    let usable = calculate_usable_context(context_window, max_output);
    // @step Then usable context should be 191808 tokens
    assert_eq!(usable, 191_808);
}

#[test]
fn test_usable_context_gpt4() {
    // @step Given a model with context_window of 128000 tokens
    let context_window = 128_000;
    // @step And the model has max_output_tokens of 4096
    let max_output = 4_096;
    // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    // @step When I calculate usable context
    let usable = calculate_usable_context(context_window, max_output);
    // @step Then usable context should be 123904 tokens
    assert_eq!(usable, 123_904);
}

#[test]
fn test_usable_context_high_output_capped() {
    // @step Given a model with context_window of 200000 tokens
    let context_window = 200_000;
    // @step And the model has max_output_tokens of 64000
    let max_output = 64_000;
    // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
    assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    // @step When I calculate usable context
    let usable = calculate_usable_context(context_window, max_output);
    // @step Then usable context should be 168000 tokens
    assert_eq!(usable, 168_000);
}

#[test]
fn test_usable_context_zero_max_output_unavailable() {
    // @step Given a model with context_window of 100000 tokens
    let context_window = 100_000;
    // @step And the model has max_output_tokens of 0
    let max_output = 0;
    // @step When I calculate usable context
    let usable = calculate_usable_context(context_window, max_output);
    // @step Then usable context should be 0 tokens
    assert_eq!(usable, 0);
    assert_eq!(
        compaction_threshold_for(context_window, max_output, None, None),
        0,
        "no usable context leaves no positive threshold"
    );
}

#[test]
fn test_token_count_includes_all_three_token_types() {
    // @step Given input_tokens of 150000
    // @step And cache_read_tokens of 20000
    // @step And output_tokens of 5000
    let state = TokenState {
        input_tokens: 150_000,
        cache_read_input_tokens: 20_000,
        output_tokens: 5_000,
        ..TokenState::default()
    };
    // @step When I calculate total token count
    let total = state.total();
    // @step Then total token count should be 175000
    assert_eq!(total, 175_000);
}

#[test]
fn test_token_count_does_not_discount_cache_tokens() {
    // @step Given input_tokens of 180000
    // @step And cache_read_tokens of 50000
    // @step And output_tokens of 10000
    let state = TokenState {
        input_tokens: 180_000,
        cache_read_input_tokens: 50_000,
        output_tokens: 10_000,
        ..TokenState::default()
    };
    // @step When I calculate total token count
    let total = state.total();
    // @step Then total token count should be 240000
    assert_eq!(total, 240_000, "cache reads count in full, not 135,000");
}

#[tokio::test]
async fn test_no_compaction_under_threshold() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    // @step And input_tokens of 130000
    // @step And cache_read_tokens of 20000
    // @step And output_tokens of 5000
    let state = TokenState {
        input_tokens: 130_000,
        cache_read_input_tokens: 20_000,
        output_tokens: 5_000,
        ..TokenState::default()
    };
    // @step When I check if compaction should trigger
    let triggered = hook_triggers(state).await;
    // @step Then compaction should NOT trigger
    assert!(!triggered, "155,000 tokens is under the 160,000 threshold");
}

#[tokio::test]
async fn test_compaction_triggers_over_threshold() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    // @step And input_tokens of 170000
    // @step And cache_read_tokens of 20000
    // @step And output_tokens of 5000
    let state = TokenState {
        input_tokens: 170_000,
        cache_read_input_tokens: 20_000,
        output_tokens: 5_000,
        ..TokenState::default()
    };
    // @step When I check if compaction should trigger
    let triggered = hook_triggers(state).await;
    // @step Then compaction should trigger
    assert!(triggered, "195,000 tokens is over the 160,000 threshold");
}

#[tokio::test]
async fn test_no_compaction_exactly_at_threshold() {
    // @step Given a Claude model with context_window of 200000 and max_output of 8192
    // @step And input_tokens of 140000
    // @step And cache_read_tokens of 15000
    // @step And output_tokens of 5000
    let state = TokenState {
        input_tokens: 140_000,
        cache_read_input_tokens: 15_000,
        output_tokens: 5_000,
        ..TokenState::default()
    };
    // @step When I check if compaction should trigger
    let triggered = hook_triggers(state).await;
    // @step Then compaction should NOT trigger
    assert!(
        !triggered,
        "exactly 160,000 tokens does not exceed the threshold"
    );
}

#[test]
fn test_calculate_budget_large_context() {
    assert_eq!(calculate_summarization_budget(200_000), 150_000);
}

#[test]
fn test_calculate_budget_small_context() {
    assert_eq!(calculate_summarization_budget(40_000), 32_000);
}

#[test]
fn test_calculate_budget_equal_to_buffer() {
    assert_eq!(calculate_summarization_budget(50_000), 40_000);
}

#[test]
fn test_compaction_threshold_large_context_model() {
    let usable = calculate_usable_context(200_000, 100_000);
    assert_eq!(usable, 168_000);
    assert!(168_001 > usable);
}

#[test]
fn test_compaction_threshold_small_context_model() {
    let usable = calculate_usable_context(32_000, 4_096);
    assert_eq!(usable, 27_904);
    assert!(27_905 > usable);
}
