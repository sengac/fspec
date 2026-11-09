use super::*;
use crate::manager::{ProviderManager, ProviderType};

// =========================================================================
// Additional edge cases: zero values, missing registry, user override priority
// =========================================================================

/// Edge case: Zero user override on a trusting provider → returns 0
#[test]
fn zero_user_override_on_trusting_provider_returns_zero() {
    let resolver = crate::gemini::GeminiProvider::from_api_key("test-key", "gemini-2.0-flash")
        .expect("Should create provider");

    // User explicitly sets 0 — no clamping since provider has no max
    let ctx = resolve_context_window(None, Some(0), &resolver);
    assert_eq!(
        ctx,
        Some(0),
        "Zero user override passes through unclamped on trusting provider"
    );
}

/// Edge case: User override below fallback defaults is retained
#[test]
fn user_override_below_fallback_defaults_is_retained() {
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    let ctx = resolve_context_window(None, Some(100_000), &resolver);
    assert_eq!(ctx, Some(100_000), "Explicit context override is retained");
}

/// Edge case: Registry and user override both present — user wins
#[test]
fn user_override_takes_priority_over_registry() {
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    // Registry says 200k, user says 150k
    let ctx = resolve_context_window(Some(200_000), Some(150_000), &resolver);
    assert_eq!(
        ctx,
        Some(150_000),
        "User override 150k wins over registry 200k"
    );
}

/// Edge case: ProviderManager override_model_limits retains explicit values
#[test]
fn provider_manager_override_model_limits_retains_explicit_values() {
    let mut manager =
        ProviderManager::for_testing(ProviderType::Claude, Some(200_000), Some(8_192));

    manager.override_model_limits(Some(500_000), Some(64_000));

    assert_eq!(
        manager.context_window(),
        Some(500_000),
        "Explicit context override is retained"
    );
    assert_eq!(
        manager.max_output_tokens(),
        Some(64_000),
        "Explicit output override is retained"
    );
}

/// Edge case: ProviderManager raw_model_context_window returns None when no data
#[test]
fn provider_manager_raw_returns_none_when_no_data() {
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);

    assert_eq!(
        manager.raw_model_context_window(),
        None,
        "No data → None for sub-agent"
    );
    assert_eq!(
        manager.raw_model_max_output_tokens(),
        None,
        "No data → None for sub-agent"
    );
}

/// Copilot with registry values → pass-through
#[test]
fn copilot_with_registry_values_passes_through() {
    let copilot =
        ProviderManager::for_testing(ProviderType::GitHubCopilot, Some(128_000), Some(16_384));
    assert_eq!(
        copilot.context_window(),
        Some(128_000),
        "Copilot passes through registry 128k"
    );
    assert_eq!(
        copilot.max_output_tokens(),
        Some(16_384),
        "Copilot passes through registry 16384"
    );
}

/// Z.AI with registry values → pass-through
#[test]
fn zai_with_registry_values_passes_through() {
    let zai = ProviderManager::for_testing(ProviderType::ZAI, Some(128_000), Some(8_192));
    assert_eq!(
        zai.context_window(),
        Some(128_000),
        "Z.AI passes through registry 128k"
    );
    assert_eq!(
        zai.max_output_tokens(),
        Some(8_192),
        "Z.AI passes through registry 8192"
    );
}

/// Codex with registry values → pass-through (no clamping)
#[test]
fn codex_with_registry_values_passes_through() {
    let codex = ProviderManager::for_testing(ProviderType::Codex, Some(300_000), Some(8_192));
    assert_eq!(
        codex.context_window(),
        Some(300_000),
        "Codex passes through registry 300k"
    );
    assert_eq!(
        codex.max_output_tokens(),
        Some(8_192),
        "Codex passes through registry 8192"
    );
}

/// OpenAI o3 with large registry values → pass-through (trusts registry)
#[test]
#[serial_test::serial]
fn openai_o3_with_large_registry_passes_through() {
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    let openai = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(100_000));
    assert_eq!(
        openai.context_window(),
        Some(200_000),
        "OpenAI o3 passes through 200k"
    );
    assert_eq!(
        openai.max_output_tokens(),
        Some(100_000),
        "OpenAI o3 passes through 100k"
    );
}
