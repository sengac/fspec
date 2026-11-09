// =========================================================================
// LIMITS-004: ProviderManager resolves through ModelLimitsResolver
// Feature: spec/features/fix-providermanager-resolution-chain-use-modellimitsresolver.feature
// =========================================================================

/// Scenario: Claude context window retains 1M registry value
#[test]
fn test_claude_context_window_retains_1m_registry_value() {
    // @step Given a ProviderManager configured for the Claude provider
    let mut manager = test_manager(ProviderType::Claude);

    // @step And select_model stores a registry context window of 1000000
    manager.registry_context_window = Some(1_000_000);

    // @step And the Claude resolver declares max_context_window as None
    assert_eq!(
        manager.provider_limits_resolver().max_context_window(),
        None
    );

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result should be 1000000
    assert_eq!(result, Some(1_000_000));
}

/// Scenario: Claude maximum output retains 128k registry value
#[test]
fn test_claude_maximum_output_retains_128k_registry_value() {
    // @step Given a ProviderManager configured for the Claude provider
    let mut manager = test_manager(ProviderType::Claude);

    // @step And select_model stores a registry max output of 128000
    manager.registry_max_output_tokens = Some(128_000);

    // @step And the Claude resolver declares max_output_tokens_limit as None
    assert_eq!(
        manager.provider_limits_resolver().max_output_tokens_limit(),
        None
    );

    // @step When max_output_tokens() is called
    let result = manager.max_output_tokens();

    // @step Then the result should be 128000
    assert_eq!(result, Some(128_000));
}

/// Scenario: OpenAI context window passes through unclamped
#[test]
fn test_openai_context_window_passes_through_unclamped() {
    // @step Given a ProviderManager configured for the OpenAI provider
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step And select_model stores a registry context window of 128000
    manager.registry_context_window = Some(128_000);

    // @step And the OpenAI resolver declares max_context_window as None
    // (built into the resolver — OpenAI trusts registry)

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result should be 128000
    assert_eq!(result, Some(128_000));
}

/// Scenario: Codex with no registry data reports an unavailable context window
#[test]
fn test_codex_no_registry_returns_default() {
    // @step Given a ProviderManager configured for the Codex provider
    let manager = test_manager(ProviderType::Codex);

    // @step And no registry context window is set
    assert_eq!(manager.registry_context_window, None);

    // @step And no user context window override is set
    assert_eq!(manager.user_context_window, None);

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result is unavailable
    // BUG-309 / PROV-165: Codex has no guessed default; limits come from the
    // catalog or are unavailable.
    assert_eq!(result, None);
}

/// Scenario: Claude user override is retained without a blanket ceiling
#[test]
fn test_claude_user_override_retained_without_blanket_ceiling() {
    // @step Given a ProviderManager configured for the Claude provider
    let mut manager = test_manager(ProviderType::Claude);

    // @step And the user overrides context window to 500000 via NAPI
    manager.override_model_limits(Some(500_000), None);

    // @step And the Claude resolver declares max_context_window as None
    assert_eq!(
        manager.provider_limits_resolver().max_context_window(),
        None
    );

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result should be 500000
    assert_eq!(result, Some(500_000));
}

/// Scenario: User override takes priority over registry value
#[test]
#[serial_test::serial]
fn test_user_override_takes_priority_over_registry() {
    // @step Given a ProviderManager configured for the OpenAI provider
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
    let mut manager = test_manager(ProviderType::OpenAI);

    // @step And select_model stores a registry context window of 128000
    manager.registry_context_window = Some(128_000);

    // @step And the user overrides context window to 64000 via NAPI
    manager.override_model_limits(Some(64_000), None);

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result should be 64000
    assert_eq!(result, Some(64_000));
}

/// Scenario: Sub-agent propagation returns effective model limits
#[test]
fn test_sub_agent_propagation_returns_effective_model_limits() {
    // @step Given a ProviderManager configured for the Claude provider
    let mut manager = test_manager(ProviderType::Claude);

    // @step And select_model stores a registry context window of 1000000
    manager.registry_context_window = Some(1_000_000);

    // @step When raw_model_context_window() is called for sub-agent propagation
    let raw = manager.raw_model_context_window();

    // @step Then the result should be 1000000
    assert_eq!(raw, Some(1_000_000));

    // @step And it should equal the value from context_window()
    assert_eq!(raw, manager.context_window());
}

/// Scenario: OpenAI env var override when no registry data
#[test]
#[serial_test::serial]
fn test_openai_env_var_override_when_no_registry() {
    // @step Given the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
    std::env::set_var("OPENAI_CONTEXT_WINDOW", "256000");

    // @step And a ProviderManager configured for the OpenAI provider
    let manager =
        with_openai_test_env(|| ProviderManager::with_provider("openai")).expect("openai manager");

    // @step And no registry context window is set
    assert_eq!(manager.registry_context_window, None);

    // @step And no user context window override is set
    assert_eq!(manager.user_context_window, None);

    // @step When context_window() is called
    let result = manager.context_window();

    // @step Then the result should be 256000
    assert_eq!(result, Some(256_000));

    std::env::remove_var("OPENAI_CONTEXT_WINDOW");
}
