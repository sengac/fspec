use super::*;
use crate::manager::{ProviderManager, ProviderType};

// Scenario: Claude Sonnet 4 resolves context window to 200k and max output to 8192

#[test]
fn claude_sonnet_4_resolves_context_200k_output_8192() {
    // @step Given the Claude provider resolver without blanket context or output ceilings
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.max_output_tokens_limit(), None);

    // @step When resolve_context_window is called with registry value 200000 and no user override
    let ctx = resolve_context_window(Some(200_000), None, &resolver);
    let out = resolve_max_output_tokens(Some(8_192), None, &resolver);

    // @step Then context_window should be 200000 and max_output_tokens should be 8192
    assert_eq!(ctx, Some(200_000));
    assert_eq!(out, Some(8_192));
}

// Scenario: Claude Opus 4.6 retains 1M context and 128k output

#[test]
fn claude_opus_4_6_retains_1m_context_and_128k_output() {
    // @step Given the Claude provider resolver without blanket context or output ceilings
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-opus-4-6-20250610",
    )
    .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.max_output_tokens_limit(), None);

    // @step When resolve_context_window is called with registry value 1000000 and no user override
    let ctx = resolve_context_window(Some(1_000_000), None, &resolver);
    let out = resolve_max_output_tokens(Some(128_000), None, &resolver);

    // @step Then context_window should be 1000000 and max_output_tokens should be 128000
    assert_eq!(ctx, Some(1_000_000));
    assert_eq!(out, Some(128_000));
}

// Scenario: OpenAI gpt-4o trusts registry values without clamping

#[test]
#[serial_test::serial]
fn openai_gpt4o_trusts_registry_values() {
    // @step Given the OpenAI provider resolver with no max_context_window and no default context window
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    let resolver =
        crate::openai::OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)
            .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.max_output_tokens_limit(), None);

    // @step When resolve_context_window is called with registry value 128000
    let ctx = resolve_context_window(Some(128_000), None, &resolver);
    let out = resolve_max_output_tokens(Some(16_384), None, &resolver);

    // @step Then context_window should be 128000 and max_output_tokens should be 16384
    assert_eq!(
        ctx,
        Some(128_000),
        "OpenAI should pass through 128k unchanged"
    );
    assert_eq!(
        out,
        Some(16_384),
        "OpenAI should pass through 16384 unchanged"
    );
}

// Scenario: Gemini 2.5 Pro trusts 1M registry value and uses 80% threshold

#[test]
fn gemini_2_5_pro_trusts_1m_registry() {
    // @step Given the Gemini provider resolver with no max_context_window and default 1000000
    let resolver = crate::gemini::GeminiProvider::from_api_key("test-key", "gemini-2.5-pro")
        .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.default_context_window(), Some(1_000_000));

    // @step When resolve_context_window is called with registry value 1000000
    let ctx = resolve_context_window(Some(1_000_000), None, &resolver);
    let out = resolve_max_output_tokens(Some(8_192), None, &resolver);

    // @step Then context_window should be 1000000 and max_output_tokens should be 8192
    assert_eq!(
        ctx,
        Some(1_000_000),
        "Gemini should trust 1M registry value"
    );
    assert_eq!(out, Some(8_192), "Gemini should trust 8192 output tokens");
}

// Scenario: Z.AI resolves 128k context window and 8192 max output

#[test]
fn zai_resolves_128k_context_8192_output() {
    // @step Given the Z.AI provider resolver with no max_context_window and default 128000
    let resolver = crate::zai::ZAIProvider::from_api_key("test-key", "glm-4-plus")
        .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.default_context_window(), Some(128_000));

    // @step When resolve_context_window is called with no registry value
    let ctx = resolve_context_window(None, None, &resolver);
    let out = resolve_max_output_tokens(None, None, &resolver);

    // @step Then context_window should be 128000 and max_output_tokens should be 8192
    assert_eq!(ctx, Some(128_000), "Z.AI should use 128k default");
    assert_eq!(out, Some(8_192), "Z.AI should use 8192 default");
}

// Scenario: Copilot falls back to 200k default context and 4096 max output

#[test]
fn copilot_falls_back_to_200k_default() {
    // @step Given the Copilot provider resolver with no max_context_window and default 200000
    let resolver = crate::copilot::CopilotProvider::new(
        crate::copilot::CopilotDeploymentType::GitHubCom,
        "ghu_test_token_12345".to_string(),
        "gpt-4o",
    )
    .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.default_context_window(), Some(200_000));

    // @step When resolve_context_window is called with no registry value
    let ctx = resolve_context_window(None, None, &resolver);
    let out = resolve_max_output_tokens(None, None, &resolver);

    // @step Then context_window should be 200000 and max_output_tokens should be 4096
    assert_eq!(ctx, Some(200_000), "Copilot should use 200k default");
    assert_eq!(out, Some(4_096), "Copilot should use 4096 default");
}

// Scenario: Claude user override above historical defaults is retained

#[test]
fn claude_user_override_above_historical_defaults_is_retained() {
    // @step Given the Claude provider resolver without a blanket context ceiling
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    // @step When a user override of 500000 is applied
    let ctx = resolve_context_window(None, Some(500_000), &resolver);
    let out = resolve_max_output_tokens(None, Some(32_000), &resolver);

    // @step Then context_window should be 500000
    assert_eq!(ctx, Some(500_000));
    assert_eq!(out, Some(32_000));
}

// Scenario: ProviderManager full chain resolves clamped values for all providers

#[test]
#[serial_test::serial]
fn provider_manager_full_chain_all_providers() {
    // @step Given a ProviderManager configured for each provider type
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    // @step When context_window() and max_output_tokens() are called with registry values
    let claude = ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(128_000));
    // @step Then each provider returns correctly clamped or pass-through values
    assert_eq!(
        claude.context_window(),
        Some(1_000_000),
        "Claude retains catalog context"
    );
    assert_eq!(
        claude.max_output_tokens(),
        Some(128_000),
        "Claude retains catalog output"
    );

    // OpenAI with registry values → pass-through
    let openai = ProviderManager::for_testing(ProviderType::OpenAI, Some(128_000), Some(16_384));
    assert_eq!(
        openai.context_window(),
        Some(128_000),
        "OpenAI: 128k pass-through"
    );
    assert_eq!(
        openai.max_output_tokens(),
        Some(16_384),
        "OpenAI: 16384 pass-through"
    );

    // Gemini with 1M registry → pass-through
    let gemini = ProviderManager::for_testing(ProviderType::Gemini, Some(1_000_000), Some(8_192));
    assert_eq!(
        gemini.context_window(),
        Some(1_000_000),
        "Gemini: 1M pass-through"
    );
    assert_eq!(
        gemini.max_output_tokens(),
        Some(8_192),
        "Gemini: 8192 pass-through"
    );

    // BUG-309: Codex without catalog data has no guessed limits.
    let codex = ProviderManager::for_testing(ProviderType::Codex, None, None);
    assert_eq!(
        codex.context_window(),
        None,
        "Codex: unavailable without catalog"
    );
    assert_eq!(
        codex.max_output_tokens(),
        None,
        "Codex: unavailable without catalog"
    );

    // Z.AI no registry → defaults
    let zai = ProviderManager::for_testing(ProviderType::ZAI, None, None);
    assert_eq!(zai.context_window(), Some(128_000), "Z.AI: 128k default");
    assert_eq!(zai.max_output_tokens(), Some(8_192), "Z.AI: 8192 default");

    // Copilot no registry → defaults
    let copilot = ProviderManager::for_testing(ProviderType::GitHubCopilot, None, None);
    assert_eq!(
        copilot.context_window(),
        Some(200_000),
        "Copilot: 200k default"
    );
    assert_eq!(
        copilot.max_output_tokens(),
        Some(4_096),
        "Copilot: 4096 default"
    );
}

// Scenario: Sub-agent propagation returns catalog context for Claude

#[test]
fn sub_agent_propagation_returns_catalog_context_for_claude() {
    // @step Given a ProviderManager with Claude and registry context_window of 1000000
    let mut manager =
        ProviderManager::for_testing(ProviderType::Claude, Some(1_000_000), Some(128_000));
    // Ensure registry values are set
    manager.registry_context_window = Some(1_000_000);
    manager.registry_max_output_tokens = Some(128_000);

    // @step When raw_model_context_window() is called for sub-agent propagation
    let raw_ctx = manager.raw_model_context_window();
    let raw_out = manager.raw_model_max_output_tokens();

    // @step Then the returned value should be 1000000
    assert_eq!(
        raw_ctx,
        Some(1_000_000),
        "Sub-agent receives effective catalog context"
    );
    assert_eq!(
        raw_out,
        Some(128_000),
        "Sub-agent receives effective catalog output"
    );
}
