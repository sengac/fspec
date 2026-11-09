use super::*;
use codelet_core::{CompactionHook, TokenState};
use codelet_providers::{ProviderManager, ProviderType};
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::message::Message;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use std::sync::{Arc, Mutex};

#[test]
fn test_limits005_claude_opus_effective_threshold() {
    // @step Given a Claude model where models.dev reports context_window of 1000000 and max_output of 16384
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    // @step When resolve_compaction_threshold is called with the effective values and model_id "claude-opus-4-6"
    let threshold = resolve_compaction_threshold(
        manager.context_window().map(|limit| limit as u64),
        manager.max_output_tokens().map(|limit| limit as u64),
        Some("claude-opus-4-6"),
        None,
    );
    // @step Then the threshold should be 800000 tokens (80 percent of 1000000)
    assert_eq!(threshold, Some(800_000));
    // @step And the Claude provider retains context_window 1000000 and max_output 16384
    assert_eq!(manager.context_window(), Some(1_000_000));
    assert_eq!(manager.max_output_tokens(), Some(16_384));
}

#[test]
fn test_limits005_context_fill_percentage_accuracy() {
    // @step Given a clamped compaction threshold of 160000 tokens
    let threshold = compaction_threshold_for(200_000, 8_192, Some("claude-sonnet-4"), None);
    assert_eq!(threshold, 160_000);
    // @step When the current token count is 87000
    let total_tokens: u64 = 87_000;
    // @step Then the fill percentage should be approximately 54 percent (not 10 percent from wrong 800k threshold)
    let fill_percentage = ((total_tokens as f64 / threshold as f64) * 100.0) as u32;
    assert_eq!(fill_percentage, 54);
    let wrong_threshold: u64 = 800_000;
    let wrong_fill = ((total_tokens as f64 / wrong_threshold as f64) * 100.0) as u32;
    assert_eq!(wrong_fill, 10);
}

#[tokio::test]
async fn test_limits005_compaction_fires_before_api_limit() {
    // @step Given a CompactionHook with threshold 160000 derived from clamped context_window
    let threshold = compaction_threshold_for(200_000, 8_192, Some("claude-sonnet-4"), None);
    assert_eq!(threshold, 160_000);
    let state = Arc::new(Mutex::new(TokenState::default()));
    let hook = CompactionHook::new(Arc::clone(&state), threshold);

    // @step When the token count reaches 160001 tokens
    if let Ok(mut current) = state.lock() {
        current.input_tokens = 160_001;
    }
    let signal = CancelSignal::new();
    <CompactionHook as StreamingPromptHook<ResponsesCompletionModel>>::on_completion_call(
        &hook,
        &Message::user("continue"),
        &[],
        signal.clone(),
    )
    .await;

    // @step Then compaction should be triggered well before the 200000 API limit
    assert!(
        signal.is_cancelled(),
        "the request must be cancelled for compaction"
    );
    assert!(
        state.lock().is_ok_and(|current| current.compaction_needed),
        "compaction_needed must be set at 160001 tokens"
    );
    assert!(hook.threshold() < 200_000);
}

#[test]
fn test_limits005_gemini_threshold() {
    // @step Given a Gemini model with context_window of 1000000 and max_output of 65536
    let (context_window, max_output) = (1_000_000, 65_536);
    // @step When compaction_threshold_for is called with model_id "gemini-2.5-pro"
    let threshold =
        compaction_threshold_for(context_window, max_output, Some("gemini-2.5-pro"), None);
    // @step Then the threshold should be 800000 tokens (80 percent of 1M)
    assert_eq!(threshold, 800_000);
}

#[test]
fn test_limits005_gpt4o_threshold() {
    // @step Given an OpenAI model with context_window of 128000 and max_output of 16384
    let (context_window, max_output) = (128_000, 16_384);
    // @step When compaction_threshold_for is called with model_id "gpt-4o"
    let threshold = compaction_threshold_for(context_window, max_output, Some("gpt-4o"), None);
    // @step Then the threshold should be 102400 tokens (80 percent of 128k)
    assert_eq!(threshold, 102_400);
}

#[test]
fn test_limits005_summarization_budget_clamped() {
    // @step Given a clamped context_window of 200000 for Claude
    let context_window = 200_000;
    // @step When calculate_summarization_budget is called with 200000
    let budget = calculate_summarization_budget(context_window);
    // @step Then the budget should be 150000 (200k minus 50k AUTOCOMPACT_BUFFER)
    assert_eq!(budget, 150_000);
}

#[test]
fn test_limits005_thinking_exhaustion_uses_effective_context() -> Result<(), &'static str> {
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    let current_tokens: u64 = 925_000;
    let context_window = manager
        .context_window()
        .ok_or("catalogued context window")?;
    let utilization_pct = (current_tokens as f64 / context_window as f64) * 100.0;
    assert!(utilization_pct > 90.0);
    assert_eq!(utilization_pct, 92.5);
    Ok(())
}

#[test]
fn test_limits005_end_to_end_claude_chain() {
    let context_window: u64 = 1_000_000;
    let threshold = compaction_threshold_for(context_window, 16_384, Some("claude-opus-4-6"), None);
    assert_eq!(threshold, 800_000);
    assert_eq!(calculate_summarization_budget(context_window), 950_000);
    let budget = calculate_summarization_budget(threshold);
    assert_eq!(budget, 750_000);
    assert_eq!(threshold - budget, 50_000);
    let fill_at_87k = ((87_000_f64 / threshold as f64) * 100.0) as u32;
    assert_eq!(fill_at_87k, 10);
    let fill_at_threshold = ((threshold as f64 / threshold as f64) * 100.0) as u32;
    assert_eq!(fill_at_threshold, 100);
}

#[test]
fn test_limits005_sub_agent_threshold_from_effective_values() {
    // @step Given a ProviderManager with registry context_window of 1000000 for Claude
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    // @step When raw_model_context_window is called for DeepSearch or AgentManager sub-agent propagation
    let sub_agent_context_window = manager.raw_model_context_window();
    // @step Then it should return 1000000 from the effective model limits
    assert_eq!(sub_agent_context_window, Some(1_000_000));
    assert_eq!(
        resolve_compaction_threshold(
            sub_agent_context_window.map(|limit| limit as u64),
            manager.max_output_tokens().map(|limit| limit as u64),
            Some("claude-opus-4-6"),
            None,
        ),
        Some(800_000)
    );
}
