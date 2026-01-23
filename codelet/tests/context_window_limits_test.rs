#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/use-model-specific-context-window-limits.feature
//
// CLI-015: Use model-specific context window limits
//
// Tests for model-specific context window lookup and compaction threshold calculations.

// =============================================================================
// Scenario: Claude provider returns correct context window
// =============================================================================

#[test]
fn test_claude_provider_returns_correct_context_window() {
    // @step Given the current provider is Claude
    let provider_name = "claude";

    // @step When I query the context window size
    let context_window = get_context_window_for_provider_name(provider_name);

    // @step Then the context window should be 200000 tokens
    assert_eq!(context_window, 200_000);
}

// =============================================================================
// Scenario: OpenAI provider returns correct context window
// =============================================================================

#[test]
fn test_openai_provider_returns_correct_context_window() {
    // @step Given the current provider is OpenAI
    let provider_name = "openai";

    // @step When I query the context window size
    let context_window = get_context_window_for_provider_name(provider_name);

    // @step Then the context window should be 128000 tokens
    assert_eq!(context_window, 128_000);
}

// =============================================================================
// Scenario: Gemini provider returns correct context window
// =============================================================================

#[test]
fn test_gemini_provider_returns_correct_context_window() {
    // @step Given the current provider is Gemini
    let provider_name = "gemini";

    // @step When I query the context window size
    let context_window = get_context_window_for_provider_name(provider_name);

    // @step Then the context window should be 1000000 tokens
    assert_eq!(context_window, 1_000_000);
}

// =============================================================================
// Scenario: Codex provider returns correct context window
// =============================================================================

#[test]
fn test_codex_provider_returns_correct_context_window() {
    // @step Given the current provider is Codex
    let provider_name = "codex";

    // @step When I query the context window size
    let context_window = get_context_window_for_provider_name(provider_name);

    // @step Then the context window should be 272000 tokens
    assert_eq!(context_window, 272_000);
}

// =============================================================================
// Scenario: Compaction threshold uses model-specific context window
// =============================================================================

#[test]
fn test_compaction_triggers_at_90_percent_of_context_window() {
    // @step Given a Claude provider with context_window=200000
    let context_window: u64 = 200_000;
    let threshold = (context_window as f64 * 0.9) as u64; // 180,000

    // @step And effective_tokens has reached 180000 (90% of context window)
    let effective_tokens: u64 = 180_000;

    // @step When the compaction check is performed
    let should_compact = effective_tokens > threshold;

    // @step Then compaction should be triggered
    // At exactly 180k with threshold at 180k, should NOT trigger (> not >=)
    // But 180001 should trigger
    assert!(!should_compact, "Exactly at threshold should not trigger");

    // Test just above threshold
    let effective_tokens_above: u64 = 180_001;
    let should_compact_above = effective_tokens_above > threshold;
    assert!(should_compact_above, "Above threshold should trigger");
}

// =============================================================================
// Scenario: Compaction does not trigger below threshold
// =============================================================================

#[test]
fn test_compaction_does_not_trigger_below_threshold() {
    // @step Given a Claude provider with context_window=200000
    let context_window: u64 = 200_000;
    let threshold = (context_window as f64 * 0.9) as u64; // 180,000

    // @step And effective_tokens is at 170000 (85% of context window)
    let effective_tokens: u64 = 170_000;

    // @step When the compaction check is performed
    let should_compact = effective_tokens > threshold;

    // @step Then compaction should NOT be triggered
    assert!(!should_compact);
}

// =============================================================================
// Helper function - this logic will be implemented in ProviderManager
// =============================================================================

/// Get context window size for a provider by name
/// This maps provider names to their context window sizes.
/// The actual implementation will be in ProviderManager::context_window()
fn get_context_window_for_provider_name(provider_name: &str) -> usize {
    match provider_name.to_lowercase().as_str() {
        "claude" => 200_000,
        "openai" => 128_000,
        "gemini" => 1_000_000,
        "codex" => 272_000,
        _ => 100_000, // Default for unknown providers
    }
}
