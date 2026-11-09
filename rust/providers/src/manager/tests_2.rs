/// Build a test registry containing an OpenAI o3 model with 200k context
/// and 100k max output, plus an anthropic/claude-sonnet-4 model.
fn build_multi_provider_registry() -> ModelRegistry {
    let mut openai_models = HashMap::new();
    openai_models.insert(
        "o3".to_string(),
        ModelInfo {
            id: "o3".to_string(),
            name: "o3".to_string(),
            family: Some("o3".to_string()),
            release_date: None,
            attachment: false,
            reasoning: true,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 200_000,
                output: 100_000,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );
    openai_models.insert(
        "gpt-4o".to_string(),
        ModelInfo {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            family: Some("gpt-4o".to_string()),
            release_date: None,
            attachment: false,
            reasoning: false,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 128_000,
                output: 16_384,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );

    let mut anthropic_models = HashMap::new();
    anthropic_models.insert(
        "claude-sonnet-4".to_string(),
        ModelInfo {
            id: "claude-sonnet-4".to_string(),
            name: "Claude Sonnet 4".to_string(),
            family: Some("claude-sonnet".to_string()),
            release_date: None,
            attachment: false,
            reasoning: false,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 200_000,
                output: 8_192,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );

    let mut copilot_models = HashMap::new();
    copilot_models.insert(
        "gemini-2.5-pro".to_string(),
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            name: "Gemini 2.5 Pro".to_string(),
            family: Some("gemini-2.5".to_string()),
            release_date: None,
            attachment: false,
            reasoning: false,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 1_000_000,
                output: 8_192,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );

    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            env: vec![],
            npm: None,
            api: Some("https://api.openai.com".to_string()),
            doc: None,
            models: openai_models,
        },
    );
    providers.insert(
        "anthropic".to_string(),
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            env: vec![],
            npm: None,
            api: Some("https://api.anthropic.com".to_string()),
            doc: None,
            models: anthropic_models,
        },
    );
    providers.insert(
        "github-copilot".to_string(),
        ProviderInfo {
            id: "github-copilot".to_string(),
            name: "GitHub Copilot".to_string(),
            env: vec![],
            npm: None,
            api: Some("https://api.githubcopilot.com".to_string()),
            doc: None,
            models: copilot_models,
        },
    );
    ModelRegistry::from_response(ModelsDevResponse { providers })
}

/// Build a test ProviderManager with multi-provider registry and
/// credentials enabled for testing.
fn test_manager_with_registry_and_credentials() -> ProviderManager {
    ProviderManager {
        runtime_options: crate::ProviderRuntimeOptions::default(),
        credentials: ProviderCredentials {
            claude_available: true,
            openai_available: true,
            codex_available: false,
            gemini_available: false,
            zai_available: false,
            github_copilot_available: true,
            custom_available: std::collections::HashMap::new(),
        },
        current_provider: ProviderType::Claude,
        model_registry: Some(build_multi_provider_registry()),
        selected_model: None,
        selected_registry_provider_id: None,
        selected_profile_name: None,
        registry_context_window: None,
        registry_max_output_tokens: None,
        user_context_window: None,
        user_max_output_tokens: None,
        facade_override: None,
        compaction_threshold_override: None,
        openai_env_limits: crate::openai::OpenAIEnvLimits::default(),
        profile_reasoning: ProfileReasoning::default(),
        codex_catalog: None,
        engine_context_window: None,
    }
}

// -------------------------------------------------------------------------
// Scenario: Cloud model gets per-model context window from models.dev registry
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_select_model_stores_model_limits_from_registry() {
    // @step Given the model registry contains "openai/o3" with context=200000 and max_output=100000
    // Need real env var for select_model's re-detect
    std::env::set_var("OPENAI_API_KEY", "fake-key-for-model-005-test");
    let mut manager = test_manager_with_registry_and_credentials();

    // @step And the OpenAI provider declares no provider-level context window
    assert_eq!(test_manager(ProviderType::OpenAI).context_window(), None);

    // @step When I call select_model("openai/o3")
    let result = manager.select_model("openai/o3");
    std::env::remove_var("OPENAI_API_KEY");
    assert!(
        result.is_ok(),
        "select_model should succeed: {:?}",
        result.err()
    );

    // @step Then model_context_window should be 200000
    assert_eq!(manager.registry_context_window, Some(200_000));

    // @step And model_max_output_tokens should be 100000
    assert_eq!(manager.registry_max_output_tokens, Some(100_000));

    // @step And context_window() should return 200000
    assert_eq!(manager.context_window(), Some(200_000));

    // @step And max_output_tokens() should return 100000
    assert_eq!(manager.max_output_tokens(), Some(100_000));
}

// -------------------------------------------------------------------------
// Scenario: Copilot proxy model gets per-model context from registry
// -------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn test_copilot_proxy_model_gets_per_model_context_from_registry() {
    // @step Given the model registry contains "github-copilot/gemini-2.5-pro" with context=1000000 and max_output=8192
    let _guard = FspecHome::redirect().expect("redirect FSPEC_HOME");
    // Install a fake copilot credential so PROV-057 re-detect finds it
    let auth = CopilotAuthJson::from_github_oauth_token(
        "fake-github-oauth-token-for-model-005".to_string(),
        None,
    );
    // Write synchronously, as the shared FspecHome fixture tests do
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(write_copilot_auth(&auth))
        .expect("write_copilot_auth should succeed");

    let mut manager = test_manager_with_registry_and_credentials();

    // @step And the Copilot provider-level constant is 200000
    assert_eq!(copilot::CONTEXT_WINDOW, 200_000);

    // @step When I call select_model("github-copilot/gemini-2.5-pro")
    let result = manager.select_model("github-copilot/gemini-2.5-pro");
    assert!(
        result.is_ok(),
        "select_model should succeed: {:?}",
        result.err()
    );

    // @step Then context_window() should return 1000000
    assert_eq!(manager.context_window(), Some(1_000_000));

    // @step And max_output_tokens() should return 8192
    assert_eq!(manager.max_output_tokens(), Some(8_192));
}

// -------------------------------------------------------------------------
// Scenario: Claude model gets per-model context from registry
// -------------------------------------------------------------------------
