impl ProviderManager {
    /// LIMITS-004: Get the resolved context window for sub-agent propagation.
    ///
    /// Returns `Some(context_window())` when any registry or user data exists,
    /// `None` when no model-specific data is available (sub-agent should use
    /// its own provider defaults).
    ///
    /// The returned value is always clamped by the provider's hard maximum.
    pub fn raw_model_context_window(&self) -> Option<usize> {
        if self.registry_context_window.is_some()
            || self.user_context_window.is_some()
            || self.runtime_options.context_window.is_some()
        {
            self.context_window()
        } else {
            None
        }
    }

    /// LIMITS-004: Get the resolved max output tokens for sub-agent propagation.
    ///
    /// Same semantics as `raw_model_context_window()` — returns clamped value
    /// or `None` when no model-specific data exists.
    pub fn raw_model_max_output_tokens(&self) -> Option<usize> {
        if self.registry_max_output_tokens.is_some()
            || self.user_max_output_tokens.is_some()
            || self.runtime_options.max_output_tokens.is_some()
        {
            self.max_output_tokens()
        } else {
            None
        }
    }

    /// MODEL-004: Get the facade override for agent loop dispatch.
    ///
    /// When `Some`, the agent loop should dispatch to this provider name
    /// instead of `current_provider_name()`. This allows custom models to
    /// route through a different provider backend.
    pub fn facade_override(&self) -> Option<&str> {
        self.facade_override.as_deref()
    }

    /// MODEL-004: Set or clear the facade override.
    pub fn set_facade_override(&mut self, facade: Option<String>) {
        self.facade_override = facade;
    }

    /// CTX-007: Get the compaction threshold override (type, value).
    /// Returns None if no user-configured override exists.
    pub fn compaction_threshold_override(&self) -> Option<(&str, u64)> {
        self.compaction_threshold_override
            .as_ref()
            .map(|(t, v)| (t.as_str(), *v))
            .or_else(|| {
                self.runtime_options
                    .compaction_threshold
                    .as_ref()
                    .map(|threshold| (threshold.kind.as_str(), threshold.value))
            })
    }

    /// CTX-007: Set the compaction threshold override.
    /// type_str should be "tokens" or "percentage".
    pub fn set_compaction_threshold_override(&mut self, config: Option<(String, u64)>) {
        self.compaction_threshold_override = config;
    }

    /// Test-only constructor that creates a ProviderManager without requiring credentials.
    /// Exposed for workspace-level integration tests that need to call methods like
    /// `max_output_tokens()` without a real provider backend.
    ///
    /// LIMITS-004: context_window and max_output_tokens are stored as registry values
    /// and resolved through the provider's ModelLimitsResolver.
    ///
    /// PROV-171: captures no OPENAI_* limit overrides; select a model to read them.
    #[doc(hidden)]
    pub fn for_testing(
        provider: ProviderType,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) -> Self {
        Self {
            runtime_options: crate::ProviderRuntimeOptions::default(),
            credentials: ProviderCredentials {
                claude_available: false,
                openai_available: false,
                codex_available: false,
                gemini_available: false,
                zai_available: false,
                github_copilot_available: false,
                custom_available: std::collections::HashMap::new(),
            },
            current_provider: provider,
            model_registry: None,
            selected_model: None,
            selected_registry_provider_id: None,
            selected_profile_name: None,
            registry_context_window: context_window,
            registry_max_output_tokens: max_output_tokens,
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

    /// Get Z.AI provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    /// Checks ZAI_PLAN_API_KEY first (for coding plan endpoint), then ZAI_API_KEY.
    pub fn get_zai(&self) -> Result<ZAIProvider, ProviderError> {
        if self.current_provider != ProviderType::ZAI {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Z.AI",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "zai",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        // Check for plan API key first, then normal API key
        let (api_key, is_plan) = if let Ok(key) = std::env::var("ZAI_PLAN_API_KEY") {
            if !key.is_empty() {
                (key, true)
            } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
                (key, false)
            } else {
                return Err(ProviderError::auth(
                    "zai",
                    "ZAI_API_KEY or ZAI_PLAN_API_KEY not set",
                ));
            }
        } else if let Ok(key) = std::env::var("ZAI_API_KEY") {
            (key, false)
        } else {
            return Err(ProviderError::auth(
                "zai",
                "ZAI_API_KEY or ZAI_PLAN_API_KEY not set",
            ));
        };
        ZAIProvider::from_api_key_with_endpoint(&api_key, &model_id, is_plan)
    }
}
