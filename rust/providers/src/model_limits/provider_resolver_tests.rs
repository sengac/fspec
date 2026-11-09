use super::*;

// -- Claude Provider Tests ----------------------------------------------

/// Scenario: Claude resolver retains catalog context window
#[test]
fn claude_resolver_retains_catalog_context_window() {
    // @step Given the Claude provider's resolver declares max_context_window as None
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);

    // @step When the registry reports a context window of 1000000
    let registry_value = Some(1_000_000);

    // @step Then the resolved context window should be 1000000
    let result = resolve_context_window(registry_value, None, &resolver);
    assert_eq!(result, Some(1_000_000));
}

/// Scenario: Claude resolver retains catalog maximum output tokens
#[test]
fn claude_resolver_retains_catalog_maximum_output_tokens() {
    // @step Given the Claude provider's resolver declares max_output_tokens_limit as None
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    assert_eq!(resolver.max_output_tokens_limit(), None);

    // @step When the registry reports max output tokens of 128000
    let registry_value = Some(128_000);

    // @step Then the resolved max output tokens should be 128000
    let result = resolve_max_output_tokens(registry_value, None, &resolver);
    assert_eq!(result, Some(128_000));
}

/// Scenario: Claude resolver returns correct defaults
#[test]
fn claude_resolver_returns_correct_defaults() {
    // @step Given the Claude provider's resolver is queried with no registry or user data
    let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
        "sk-ant-api03-test-key",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create provider");

    // @step Then the default context window should be 200000
    assert_eq!(resolver.default_context_window(), Some(200_000));

    // @step And the default max output tokens should be 8192
    assert_eq!(resolver.default_max_output_tokens(), Some(8_192));

    // @step And should_send_max_output_tokens should be true
    assert!(resolver.should_send_max_output_tokens());
}

// -- OpenAI Provider Tests ----------------------------------------------

/// Scenario: OpenAI resolver trusts registry values
#[test]
fn openai_resolver_trusts_registry_values() {
    // @step Given the OpenAI provider's resolver declares max_context_window as None
    let resolver =
        crate::openai::OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)
            .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);
    assert_eq!(resolver.max_output_tokens_limit(), None);

    // @step When the registry reports a context window of 128000 and max output tokens of 16384
    let ctx_result = resolve_context_window(Some(128_000), None, &resolver);
    let out_result = resolve_max_output_tokens(Some(16_384), None, &resolver);

    // @step Then the resolved context window should be 128000
    assert_eq!(ctx_result, Some(128_000));

    // @step And the resolved max output tokens should be 16384
    assert_eq!(out_result, Some(16_384));
}

/// Scenario: OpenAI resolver reads default from OPENAI_CONTEXT_WINDOW env var
#[test]
#[serial_test::serial]
fn openai_resolver_reads_context_window_env_var() {
    // @step Given the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
    std::env::set_var("OPENAI_CONTEXT_WINDOW", "256000");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    // @step When the OpenAI resolver is queried with no registry data
    let resolver =
        crate::openai::OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)
            .expect("Should create provider");

    // @step Then the default context window should be 256000
    assert_eq!(resolver.default_context_window(), Some(256_000));

    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
}

/// Scenario: OpenAI resolver reads default from OPENAI_MAX_OUTPUT_TOKENS env var
#[test]
#[serial_test::serial]
fn openai_resolver_reads_max_output_tokens_env_var() {
    // @step Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to 16384
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "16384");
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");

    // @step When the OpenAI resolver is queried with no registry data
    let resolver =
        crate::openai::OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)
            .expect("Should create provider");

    // @step Then the default max output tokens should be 16384
    assert_eq!(resolver.default_max_output_tokens(), Some(16_384));

    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
}

/// Scenario: OpenAI resolver without env overrides declares no defaults
#[test]
#[serial_test::serial]
fn openai_resolver_without_env_overrides_declares_no_defaults() {
    // @step Given no OPENAI_CONTEXT_WINDOW or OPENAI_MAX_OUTPUT_TOKENS env vars are set
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    // @step When the OpenAI resolver is queried with no registry data
    let resolver =
        crate::openai::OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)
            .expect("Should create provider");

    // @step Then the default context window should be unavailable
    assert_eq!(resolver.default_context_window(), None);

    // @step And the default max output tokens should be unavailable
    assert_eq!(resolver.default_max_output_tokens(), None);

    // @step And should_send_max_output_tokens should be true
    assert!(resolver.should_send_max_output_tokens());
}

// -- Gemini Provider Tests ----------------------------------------------

/// Scenario: Gemini resolver trusts registry and has correct defaults
#[test]
fn gemini_resolver_trusts_registry_and_has_correct_defaults() {
    // @step Given the Gemini provider's resolver declares max_context_window as None
    let resolver = crate::gemini::GeminiProvider::from_api_key("test-key", "gemini-2.0-flash")
        .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);

    // @step Then the default context window should be 1000000
    assert_eq!(resolver.default_context_window(), Some(1_000_000));

    // @step And the default max output tokens should be 8192
    assert_eq!(resolver.default_max_output_tokens(), Some(8_192));

    // @step And should_send_max_output_tokens should be true
    assert!(resolver.should_send_max_output_tokens());
}

// -- Codex Provider Tests -----------------------------------------------

/// Scenario: Codex resolver does not clamp registry values
#[test]
fn codex_resolver_does_not_clamp_registry_values() {
    // @step Given the Codex provider's resolver declares max_context_window as None
    let resolver =
        crate::codex::CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
            .expect("Should create provider");

    assert_eq!(resolver.max_context_window(), None);

    // @step When the registry reports a context window of 300000
    let result = resolve_context_window(Some(300_000), None, &resolver);

    // @step Then the resolved context window should be 300000
    assert_eq!(result, Some(300_000));
}

// -- Z.AI Provider Tests ------------------------------------------------

/// Scenario: Z.AI resolver returns correct defaults
#[test]
fn zai_resolver_returns_correct_defaults() {
    // @step Given the Z.AI provider's resolver is queried
    let resolver = crate::zai::ZAIProvider::from_api_key("test-key", "glm-4.7")
        .expect("Should create provider");

    // @step Then the default context window should be 128000
    assert_eq!(resolver.default_context_window(), Some(128_000));

    // @step And the default max output tokens should be 8192
    assert_eq!(resolver.default_max_output_tokens(), Some(8_192));

    // @step And should_send_max_output_tokens should be true
    assert!(resolver.should_send_max_output_tokens());
}

// -- Copilot Provider Tests ---------------------------------------------

/// Scenario: Copilot resolver returns correct defaults
#[test]
fn copilot_resolver_returns_correct_defaults() {
    // @step Given the Copilot provider's resolver is queried
    let resolver = crate::copilot::CopilotProvider::new(
        crate::copilot::CopilotDeploymentType::GitHubCom,
        "ghu_test_token_12345".to_string(),
        "gpt-4o",
    )
    .expect("Should create provider");

    // @step Then the default context window should be 200000
    assert_eq!(resolver.default_context_window(), Some(200_000));

    // @step And the default max output tokens should be 4096
    assert_eq!(resolver.default_max_output_tokens(), Some(4_096));

    // @step And should_send_max_output_tokens should be true
    assert!(resolver.should_send_max_output_tokens());
}
