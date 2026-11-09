#[test]
#[serial_test::serial]
fn test_claude_model_gets_per_model_context_from_registry() {
    // @step Given the model registry contains "anthropic/claude-sonnet-4" with context=200000 and max_output=8192
    std::env::set_var("ANTHROPIC_API_KEY", "fake-key-for-model-005-test");
    let mut manager = test_manager_with_registry_and_credentials();

    // @step When I call select_model("anthropic/claude-sonnet-4")
    let result = manager.select_model("anthropic/claude-sonnet-4");
    std::env::remove_var("ANTHROPIC_API_KEY");
    assert!(
        result.is_ok(),
        "select_model should succeed: {:?}",
        result.err()
    );

    // @step Then model_context_window should be 200000
    assert_eq!(manager.registry_context_window, Some(200_000));

    // @step And context_window() should return 200000
    assert_eq!(manager.context_window(), Some(200_000));
}

// -------------------------------------------------------------------------
// Scenario: No model selected falls back to provider constant
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_context_window_falls_back_to_provider_constant() {
    // @step Given a fresh ProviderManager with Claude as the current provider
    let manager = test_manager(ProviderType::Claude);

    // @step And no model has been selected
    assert!(manager.selected_model.is_none());

    // @step Then model_context_window should be None
    assert_eq!(manager.registry_context_window, None);

    // @step And context_window() should return 200000
    assert_eq!(manager.context_window(), Some(claude::CONTEXT_WINDOW));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(claude::MAX_OUTPUT_TOKENS));
}

// -------------------------------------------------------------------------
// Scenario: context_window returns model-specific value when set
// -------------------------------------------------------------------------

#[test]
fn test_context_window_returns_model_specific_value() {
    // @step Given a ProviderManager with model_context_window=200000
    let mut manager = test_manager(ProviderType::OpenAI);
    manager.registry_context_window = Some(200_000);

    // @step Then context_window() should return 200000
    assert_eq!(manager.context_window(), Some(200_000));
}

// -------------------------------------------------------------------------
// Scenario: max_output_tokens returns model-specific value when set
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_max_output_tokens_returns_model_specific_value() {
    // @step Given a ProviderManager with model_max_output_tokens=100000
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    let mut manager = test_manager(ProviderType::OpenAI);
    manager.registry_max_output_tokens = Some(100_000);

    // @step Then max_output_tokens() should return 100000
    assert_eq!(manager.max_output_tokens(), Some(100_000));
}

// -------------------------------------------------------------------------
// Scenario: Environment variable override still works when no per-model data
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_env_var_override_works_when_no_per_model_data() {
    // @step Given OPENAI_CONTEXT_WINDOW is set to "32000"
    std::env::set_var("OPENAI_CONTEXT_WINDOW", "32000");

    // @step And OPENAI_MAX_OUTPUT_TOKENS is set to "8192"
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "8192");

    // @step When a ProviderManager is created with OpenAI as the current provider
    let manager =
        with_openai_test_env(|| ProviderManager::with_provider("openai")).expect("openai manager");

    // @step And no model has been selected
    assert!(manager.selected_model.is_none());

    // @step Then model_context_window should be None
    assert_eq!(manager.registry_context_window, None);

    // @step And context_window() should return 32000
    assert_eq!(manager.context_window(), Some(32_000));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(8_192));

    // Clean up
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
}

// -------------------------------------------------------------------------
// Scenario: set_model_direct stores optional context params
// -------------------------------------------------------------------------

#[test]
fn test_set_model_direct_stores_optional_context_params() {
    // @step Given a vLLM profile model with ModelSelection.contextWindow=32000 and maxOutput=4096
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096
    let result =
        manager.set_model_direct("openai", "my-local-model", Some(32_000), Some(4_096), None);
    assert!(result.is_ok());

    // @step Then set_model_direct stores model_context_window=32000 and model_max_output_tokens=4096
    assert_eq!(manager.user_context_window, Some(32_000));
    assert_eq!(manager.user_max_output_tokens, Some(4_096));

    // @step And context_window() should return 32000
    assert_eq!(manager.context_window(), Some(32_000));

    // @step And max_output_tokens() should return 4096
    assert_eq!(manager.max_output_tokens(), Some(4_096));
}

// -------------------------------------------------------------------------
// Scenario: Codex model gets context window through NAPI parameters
// -------------------------------------------------------------------------

#[test]
fn test_codex_model_gets_context_window_through_napi_params() {
    // @step Given a Codex model with ModelSelection.contextWindow=272000 and maxOutput=4096
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When sessionSetModelProfile is called with context_window=272000 and max_output_tokens=4096
    let result =
        manager.set_model_direct("openai", "codex-model", Some(272_000), Some(4_096), None);
    assert!(result.is_ok());

    // @step Then set_model_direct stores model_context_window=272000 and model_max_output_tokens=4096
    assert_eq!(manager.user_context_window, Some(272_000));
    assert_eq!(manager.user_max_output_tokens, Some(4_096));

    // @step And context_window() should return 272000
    assert_eq!(manager.context_window(), Some(272_000));
}

// -------------------------------------------------------------------------
// Scenario: set_model_direct without context params leaves None
// -------------------------------------------------------------------------

#[test]
fn test_set_model_direct_without_context_params_leaves_none() {
    // @step Given a ProviderManager
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step When set_model_direct is called without context params
    let result = manager.set_model_direct("openai", "my-local-model", None, None, None);
    assert!(result.is_ok());

    // @step Then model_context_window should remain None
    assert_eq!(manager.user_context_window, None);

    // @step And model_max_output_tokens should remain None
    assert_eq!(manager.user_max_output_tokens, None);
}

// -------------------------------------------------------------------------
// Scenario: for_testing constructor with custom context window
// -------------------------------------------------------------------------

#[test]
fn test_for_testing_with_custom_context() {
    // @step Given I create a test ProviderManager via for_testing(OpenAI, context_window=200000, max_output_tokens=100000)
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(100_000));

    // @step Then context_window() should return 200000
    assert_eq!(manager.context_window(), Some(200_000));

    // @step And max_output_tokens() should return 100000
    assert_eq!(manager.max_output_tokens(), Some(100_000));
}

// -------------------------------------------------------------------------
// Scenario: with_provider_and_model accepts optional context window parameters
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_with_provider_and_model_accepts_context_params() {
    // @step Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4", context_window=200000, max_output_tokens=8192)
    std::env::set_var("ANTHROPIC_API_KEY", "test-key-for-provider-manager");
    let result = ProviderManager::with_provider_and_model(
        "claude",
        Some("claude-sonnet-4"),
        Some(200_000),
        Some(8_192),
    );
    std::env::remove_var("ANTHROPIC_API_KEY");

    assert!(
        result.is_ok(),
        "with_provider_and_model should succeed: {:?}",
        result.err()
    );
    let manager = result.unwrap();

    // @step Then context_window() should return 200000
    assert_eq!(manager.context_window(), Some(200_000));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(8_192));
}

// -------------------------------------------------------------------------
// Scenario: with_provider_and_model without context params falls back to provider constant
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_with_provider_and_model_without_context_falls_back() {
    // @step Given I create a ProviderManager via with_provider_and_model("claude", "claude-sonnet-4") with no context params
    std::env::set_var("ANTHROPIC_API_KEY", "test-key-for-provider-manager");
    let result =
        ProviderManager::with_provider_and_model("claude", Some("claude-sonnet-4"), None, None);
    std::env::remove_var("ANTHROPIC_API_KEY");

    assert!(result.is_ok());
    let manager = result.unwrap();

    // @step Then context_window() should return 200000
    assert_eq!(manager.context_window(), Some(claude::CONTEXT_WINDOW));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(claude::MAX_OUTPUT_TOKENS));
}

// -------------------------------------------------------------------------
// Scenario: NAPI override takes priority over models.dev metadata
// -------------------------------------------------------------------------
