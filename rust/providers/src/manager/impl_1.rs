impl ProviderManager {
    /// Create new ProviderManager, resolving the provider only when the choice
    /// is unambiguous.
    ///
    /// PROV-101: there is no priority-chain default. With a single credentialed
    /// provider that provider is used; with several, this errors rather than
    /// silently picking Claude (callers must select explicitly).
    pub fn new() -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();

        if !credentials.has_any() {
            return Err(ProviderError::auth(
                "manager",
                "No provider credentials found. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, \
                 GOOGLE_GENERATIVE_AI_API_KEY, or run 'codex auth login' to authenticate.",
            ));
        }

        let current_provider =
            crate::provider_resolution::resolve_unambiguous_provider(&credentials)?;

        let openai_env_limits = runtime::openai_env_limits_for(&current_provider)?;
        Ok(Self {
            runtime_options: crate::runtime_options::load_runtime_options(
                current_provider.as_str(),
                None,
                None,
                None,
            )?,
            credentials,
            current_provider,
            model_registry: crate::selection_metadata::cached_registry(),
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
            openai_env_limits,
            profile_reasoning: ProfileReasoning::default(),
            codex_catalog: None,
            engine_context_window: None,
        })
    }

    /// Create ProviderManager with explicit provider selection
    pub fn with_provider(provider_name: &str) -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        if !requested_provider.has_credentials(&credentials) {
            let available = credentials.available_providers();
            return Err(ProviderError::auth(
                provider_name,
                format!(
                    "Provider {} not available. Available providers: {}",
                    provider_name,
                    available.join(", ")
                ),
            ));
        }

        let openai_env_limits = runtime::openai_env_limits_for(&requested_provider)?;
        Ok(Self {
            runtime_options: crate::runtime_options::load_runtime_options(
                requested_provider.as_str(),
                None,
                None,
                None,
            )?,
            credentials,
            current_provider: requested_provider,
            model_registry: crate::selection_metadata::cached_registry(),
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
            registry_context_window: None,
            registry_max_output_tokens: None,
            user_context_window: None,
            user_max_output_tokens: None,
            facade_override: None,
            compaction_threshold_override: None,
            openai_env_limits,
            profile_reasoning: ProfileReasoning::default(),
            codex_catalog: None,
            engine_context_window: None,
        })
    }

    /// Create ProviderManager with explicit provider and model selection
    ///
    /// This is used for internal operations (like compaction) where we need to
    /// recreate a provider manager with the same settings without async model registry.
    /// The model_id is stored directly without registry validation.
    ///
    /// LIMITS-004: `with_provider_and_model` receives already-resolved values
    /// from a parent ProviderManager (via `context_window()` / `max_output_tokens()`).
    /// These are stored as registry values (already clamped by the parent).
    pub fn with_provider_and_model(
        provider_name: &str,
        model_id: Option<&str>,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) -> Result<Self, ProviderError> {
        let mut manager = Self::with_provider(provider_name)?;
        if let Some(model) = model_id {
            manager.set_model_direct(
                provider_name,
                model,
                context_window,
                max_output_tokens,
                None,
            )?;
        } else {
            manager.override_model_limits(context_window, max_output_tokens);
        }
        Ok(manager)
    }

    /// MODEL-001: Create ProviderManager with model registry support
    ///
    /// This async constructor initializes the model cache and registry,
    /// enabling dynamic model selection via `--model provider/model-id`.
    pub async fn with_model_support() -> Result<Self, ProviderError> {
        let credentials = ProviderCredentials::detect();

        // PROV-141: construction does NOT gate on `credentials.has_any()`.
        // A registry-backed manager makes NO provider selection at
        // construction — every caller immediately applies an EXPLICIT model
        // via `select_model` / `set_model_direct`, and credential validation
        // is per-model, not per-construction:
        // - cloud registry models are validated in `select_model` (which
        //   re-detects credentials and errors if the provider has none);
        // - profile models bridge their own `apiKey`/`baseUrl` from
        //   `fspec-config.json` into `OPENAI_*` env vars via
        //   `apply_profile_env_vars` and intentionally skip credential
        //   validation in `set_model_direct`;
        // - codex/custom models resolve their own credentials.
        //
        // Gating construction on a global credential set broke session
        // creation on machines with no global API keys (e.g. Linux) whenever
        // the default model was a self-sufficient profile model — it only
        // worked where a `.env` file happened to provide `OPENAI_API_KEY`.

        // PROV-101: a registry-backed manager makes NO provider selection at
        // construction — every caller of `with_model_support()` immediately
        // applies an EXPLICIT model via `select_model` / `set_model_direct`,
        // which overwrites `current_provider`. We therefore must NOT run the
        // (now-removed) priority-chain default here, and must NOT error on an
        // ambiguous credential set: the explicit model that follows IS the
        // selection. A neutral deferred placeholder is used until then.
        let current_provider = Self::deferred_placeholder_provider(&credentials);

        // Initialize model cache and registry
        let cache = ModelCache::new()?;
        let registry = ModelRegistry::new(&cache).await?;

        Ok(Self {
            credentials,
            current_provider,
            model_registry: Some(registry),
            runtime_options: crate::ProviderRuntimeOptions::default(),
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
        })
    }
}
