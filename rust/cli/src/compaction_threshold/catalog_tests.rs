use super::*;
use codelet_providers::{ProviderManager, ProviderType};

#[test]
fn test_limits005_claude_opus_effective_threshold() {
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    let threshold = resolve_compaction_threshold(
        manager.context_window() as u64,
        manager.max_output_tokens() as u64,
        Some("claude-opus-4-6"),
        None,
    );
    assert_eq!(threshold, 800_000);
    assert_eq!(manager.context_window(), 1_000_000);
    assert_eq!(manager.max_output_tokens(), 16_384);
}

#[test]
fn test_limits005_context_fill_percentage_accuracy() {
    let threshold = resolve_compaction_threshold(200_000, 8_192, Some("claude-sonnet-4"), None);
    let total_tokens: u64 = 87_000;
    let fill_percentage = ((total_tokens as f64 / threshold as f64) * 100.0) as u32;
    assert_eq!(fill_percentage, 54);
    let wrong_threshold: u64 = 800_000;
    let wrong_fill = ((total_tokens as f64 / wrong_threshold as f64) * 100.0) as u32;
    assert_eq!(wrong_fill, 10);
}

#[test]
fn test_limits005_compaction_fires_before_api_limit() {
    let threshold = resolve_compaction_threshold(200_000, 8_192, Some("claude-sonnet-4"), None);
    let token_count: u64 = 160_001;
    assert!(token_count > threshold);
    assert!(threshold < 200_000);
    let wrong_threshold: u64 = 800_000;
    assert!(token_count < wrong_threshold);
}

#[test]
fn test_limits005_gemini_threshold() {
    assert_eq!(
        resolve_compaction_threshold(1_000_000, 65_536, Some("gemini-2.5-pro"), None),
        800_000
    );
}

#[test]
fn test_limits005_gpt4o_threshold() {
    assert_eq!(
        resolve_compaction_threshold(128_000, 16_384, Some("gpt-4o"), None),
        102_400
    );
}

#[test]
fn test_limits005_summarization_budget_clamped() {
    assert_eq!(calculate_summarization_budget(200_000), 150_000);
}

#[test]
fn test_limits005_thinking_exhaustion_uses_effective_context() {
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    let current_tokens: u64 = 925_000;
    let utilization_pct = (current_tokens as f64 / manager.context_window() as f64) * 100.0;
    assert!(utilization_pct > 90.0);
    assert_eq!(utilization_pct, 92.5);
}

#[test]
fn test_limits005_end_to_end_claude_chain() {
    let context_window: u64 = 1_000_000;
    let threshold =
        resolve_compaction_threshold(context_window, 16_384, Some("claude-opus-4-6"), None);
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
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(16_384));
    let sub_agent_context_window = manager.raw_model_context_window();
    assert_eq!(sub_agent_context_window, Some(1_000_000));
    assert_eq!(
        resolve_compaction_threshold(
            sub_agent_context_window.unwrap_or_default() as u64,
            manager.max_output_tokens() as u64,
            Some("claude-opus-4-6"),
            None,
        ),
        800_000
    );
}
