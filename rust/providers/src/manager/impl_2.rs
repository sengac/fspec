impl ProviderManager {
    /// MODEL-001: Select a model using provider/model-id format
    ///
    /// Parses the model string, validates the provider exists and has credentials,
    /// validates the model exists in the registry, and ensures tool_call capability.
    ///
    /// # Arguments
    /// * `model_string` - Model in "provider/model-id" format (e.g., "anthropic/claude-sonnet-4")
    ///
    /// # Returns
    /// The validated ModelInfo for the selected model
    pub fn select_model(&mut self, model_string: &str) -> Result<&ModelInfo, ProviderError> {
        // PROV-057 (stale-cache fix): `ProviderManager::new()` snapshots
        // `ProviderCredentials::detect()` once at construction, which means
        // a credential file (e.g. `copilot_auth.json`) written *after* the
        // manager was built — typically during an in-session OAuth login —
        // remains invisible until process restart. Re-detect before the
        // `has_credentials` check so a freshly-completed Copilot login is
        // honoured by the very next `select_model` call in the same
        // session. See PROV-057 investigation §8 and §10 row 7.
        self.credentials = ProviderCredentials::detect();

        let registry = self.model_registry.as_ref().ok_or_else(|| {
            ProviderError::config(
                "manager",
                "Model registry not initialized. Use with_model_support() for model selection.",
            )
        })?;

        // Parse the model string into provider/model
        let (provider_id, model_id) =
            registry.parse_model_string(model_string).or_else(|error| {
                let (provider, model) = model_string.split_once('/').ok_or(error)?;
                let catalog = crate::selection_metadata::catalog_provider_id(provider);
                registry
                    .parse_model_string(&format!("{catalog}/{model}"))
                    .map(|_| (provider.to_string(), model.to_string()))
            })?;

        // Map models.dev provider ID to our ProviderType
        let provider_type = Self::map_provider_id_to_type(&provider_id)?;

        // Validate we have credentials for this provider
        if !provider_type.has_credentials(&self.credentials) {
            return Err(ProviderError::auth(
                &provider_id,
                format!(
                    "Provider '{}' requires credentials. Available providers: {}",
                    provider_id,
                    self.credentials.available_providers().join(", ")
                ),
            ));
        }

        // Validate model exists and has tool_call capability
        let model_info = registry
            .validate_model_for_use(
                crate::selection_metadata::catalog_provider_id(&provider_id),
                &model_id,
            )
            .or_else(|_| registry.validate_model_for_use(&provider_id, &model_id))?;
        let metadata = crate::selection_metadata::selection_metadata(
            &provider_type,
            &provider_id,
            &model_info.id,
            None,
        );
        let mut runtime_options = crate::runtime_options::load_runtime_options(
            provider_type.as_str(),
            None,
            Some(&metadata.api_model),
            metadata.model_options.as_ref(),
        )?;
        let script_compaction =
            runtime::apply_script_limits(&provider_type, &metadata, &mut runtime_options);
        let openai_env_limits = runtime::openai_env_limits_for(&provider_type)?;

        // LIMITS-004: Store raw registry values. Clamping happens at resolution time
        // via the provider's ModelLimitsResolver in context_window() / max_output_tokens().
        self.registry_context_window = model_info.limit.context_window().map(|v| v as usize);
        self.registry_max_output_tokens = model_info.limit.max_output_tokens().map(|v| v as usize);
        // Clear any previous user overrides — new model selection resets them.
        self.user_context_window = None;
        self.user_max_output_tokens = None;

        // Update state. BUG-136: store the bare model slug in
        // `selected_model` and the registry provider id in
        // `selected_registry_provider_id`. The `provider/model` composite is
        // rebuilt on demand by `selected_model_string()`. Keeping the two
        // pieces separate is essential for providers whose model ids
        // themselves contain slashes (Fireworks AI, OpenRouter, etc.) where
        // the composite is otherwise ambiguous to parse.
        self.current_provider = provider_type;
        self.selected_model = Some(model_id.to_string());
        self.selected_registry_provider_id = Some(provider_id.to_string());
        // BUG-137: Cloud model selection has no profile. Clear any stale
        // profile name left over from a previous `set_model_direct_with_profile`
        // call so `selected_model_string()` emits the plain
        // `"provider/model"` composite.
        self.selected_profile_name = None;
        self.runtime_options = runtime_options;
        self.facade_override = metadata.facade;
        self.compaction_threshold_override = script_compaction;
        self.openai_env_limits = openai_env_limits;

        Ok(model_info)
    }

    /// PROV-007: Select a model directly without registry validation
    ///
    /// For profile-based models (vLLM, Ollama, etc.) that aren't in the models.dev registry.
    /// This assumes the caller has already validated the model exists on the remote server
    /// AND has set up the required environment variables (OPENAI_API_KEY, OPENAI_BASE_URL).
    ///
    /// NOTE: This skips credentials validation because profile credentials are passed via
    /// environment variables that were set AFTER the session was created.
    ///
    /// MODEL-005: Accepts optional context_window and max_output_tokens since profile
    /// models have no registry to look up metadata — these come from the NAPI call.
    ///
    /// MODEL-004: Accepts optional facade_override for custom models that need
    /// agent loop dispatch through a different provider (e.g., a model defined in
    /// a vLLM profile that should use the Codex tool schema).
    ///
    /// # Arguments
    /// * `provider_id` - The provider ID (e.g., "openai" for OpenAI-compatible APIs)
    /// * `model_id` - The model ID as recognized by the remote server
    /// * `context_window` - Optional per-model context window size
    /// * `max_output_tokens` - Optional per-model max output tokens
    /// * `facade_override` - Optional provider name to dispatch to instead of provider_id
    pub fn set_model_direct(
        &mut self,
        provider_id: &str,
        model_id: &str,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
        facade_override: Option<String>,
    ) -> Result<(), ProviderError> {
        // BUG-137: Delegate to the profile-aware variant with profile=None.
        // Existing call sites (codex, custom providers, unit tests, session
        // initialization) behave exactly as before; only the new
        // profile-aware NAPI path passes a profile name.
        self.set_model_direct_with_profile(
            provider_id,
            model_id,
            None,
            context_window,
            max_output_tokens,
            facade_override,
        )
    }

    /// BUG-137: Variant of `set_model_direct` that also records a profile
    /// name.
    ///
    /// When `profile_name` is `Some(_)`, `selected_model_string()` emits
    /// the profile-qualified composite `"{provider}:{profile}/{model}"`
    /// (e.g. `"openai:fireworks/accounts/fireworks/models/kimi-k2p6"`).
    /// When `None`, the emitted composite is the plain `"{provider}/{model}"`
    /// — identical to the historical `set_model_direct` behaviour.
    ///
    /// This preserves the profile selection across the AgentManager spawn
    /// round-trip so subordinate sessions created via
    /// `create_session_with_id()` detect the profile format (colon before
    /// first slash) and route through `set_model_direct` again instead
    /// of failing registry validation in `select_model` with
    /// `"Model 'accounts/fireworks/...' not found in provider 'openai'"`.
    pub fn set_model_direct_with_profile(
        &mut self,
        provider_id: &str,
        model_id: &str,
        profile_name: Option<&str>,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
        facade_override: Option<String>,
    ) -> Result<(), ProviderError> {
        // Map provider ID to our ProviderType
        let provider_type = Self::map_provider_id_to_type(provider_id)?;

        // NOTE: We intentionally skip credentials validation here.
        // For profile-based models, TypeScript sets OPENAI_API_KEY and OPENAI_BASE_URL
        // as environment variables AFTER the session was created. The OpenAIProvider
        // will read these from the environment when get_openai() is called.

        // BUG-136: Store the bare model id in `selected_model` and the
        // registry provider id in `selected_registry_provider_id` —
        // `selected_model_string()` rebuilds the composite on demand.
        // Previously this path stored only `model_id`, so
        // `selected_model_string()` returned an incomplete string with no
        // provider prefix. When `AgentManager` captured that string and
        // passed it to `create_session_with_id()` the first slash segment
        // was mis-parsed as the provider name — fine for bare model ids,
        // but broken for Fireworks/OpenRouter-style ids that contain
        // slashes (e.g. "accounts/fireworks/models/kimi-k2-06-instruct").
        self.current_provider = provider_type;
        self.selected_model = Some(model_id.to_string());
        self.selected_registry_provider_id = Some(provider_id.to_string());
        // BUG-137: Store (or clear) the profile name so
        // `selected_model_string()` can emit the profile-qualified
        // composite.
        self.selected_profile_name = profile_name.map(String::from);
        // MODEL-016: a new selection declares nothing until its profile is read.
        self.profile_reasoning = ProfileReasoning::default();
        // LIMITS-004: Profile models pass values as user overrides (from NAPI).
        // They have no registry data. The resolver clamps them at resolution time.
        self.user_context_window = context_window;
        self.user_max_output_tokens = max_output_tokens;
        // MODEL-004: Store facade override for agent loop dispatch
        self.facade_override = facade_override;
        self.refresh_runtime_options()
    }

    /// Get the selected model id (the actual API model id).
    ///
    /// If a model registry is available, a registry lookup is performed so
    /// slug selections (e.g. "claude-sonnet-4") resolve to their versioned
    /// API id (e.g. "claude-sonnet-4-20250514"). Otherwise the bare model id
    /// stored by `select_model()` / `set_model_direct()` /
    /// `with_provider_and_model()` is returned verbatim.
    ///
    /// BUG-136: This method now returns exactly the bare model id — it no
    /// longer needs to strip a provider prefix, because the provider id is
    /// tracked separately in `selected_registry_provider_id`. Model ids
    /// that contain slashes (e.g.
    /// "accounts/fireworks/models/kimi-k2-06-instruct") pass through
    /// unchanged.
    pub fn selected_model_id(&self) -> Option<String> {
        let model_id = self.selected_model.as_ref()?;
        if !matches!(&self.current_provider, ProviderType::Custom(_)) {
            if let Some(model) = self.selection_model_info() {
                return Some(model.id.clone());
            }
        }
        Some(
            crate::selection_metadata::selection_metadata(
                &self.current_provider,
                self.selected_registry_provider_id
                    .as_deref()
                    .unwrap_or_else(|| self.current_provider.as_str()),
                model_id,
                self.facade_override.as_deref(),
            )
            .api_model,
        )
    }

    /// MODEL-001: Get model info for the selected model, if known to the
    /// registry.
    pub fn selected_model_info(&self) -> Option<&ModelInfo> {
        self.selection_model_info()
    }

    /// The local-server profile the selected model belongs to (BUG-137),
    /// `None` for catalog, Codex and custom-provider models. MODEL-010: a
    /// profile model takes the local-openai thinking wire.
    pub fn selected_profile_name(&self) -> Option<&str> {
        self.selected_profile_name.as_deref()
    }
}
