impl ProviderManager {
    /// MODEL-001: Get the registry-format model string ("provider/model").
    ///
    /// BUG-136: The composite is rebuilt on demand from
    /// `selected_registry_provider_id` + `selected_model`. Returning an
    /// owned `String` (rather than the previous borrowed `&str`) is
    /// necessary because the composite may not be stored as a single field.
    /// Model ids that contain slashes round-trip cleanly because the
    /// provider slug is tracked independently.
    ///
    /// BUG-137: When a profile name has been recorded (via
    /// `set_model_direct_with_profile`), the composite takes the
    /// profile-qualified form `"{provider}:{profile}/{model}"` — e.g.
    /// `"openai:fireworks/accounts/fireworks/models/kimi-k2p6"`. This
    /// matches the format produced by the TypeScript
    /// `buildModelString()` helper and consumed by
    /// `SessionManager::create_session_with_id()`, whose profile-model
    /// branch is triggered by finding `':'` before the first `'/'`.
    /// Without this, `AgentManager.spawn` captured a plain
    /// `"openai/accounts/..."` string and the subordinate path treated
    /// the model as a cloud model, failing registry validation.
    pub fn selected_model_string(&self) -> Option<String> {
        let provider_id = self.selected_registry_provider_id.as_deref()?;
        let model_id = self.selected_model.as_deref()?;
        match self.selected_profile_name.as_deref() {
            Some(profile) => Some(format!("{provider_id}:{profile}/{model_id}")),
            None => Some(format!("{provider_id}/{model_id}")),
        }
    }

    /// MODEL-001: Get the model registry (for CLI commands like `codelet models`)
    pub fn model_registry(&self) -> Option<&ModelRegistry> {
        self.model_registry.as_ref()
    }

    /// MODEL-001: Map models.dev provider ID to our ProviderType
    fn map_provider_id_to_type(provider_id: &str) -> Result<ProviderType, ProviderError> {
        // PROV-085: Shadowing precedence also applies to models.dev IDs
        // so that `--model <shadowing-slug>/<model>` routes through the
        // custom config. The same escape hatch
        // `FSPEC_DISABLE_SCRIPT_SHADOWING=1` disables this lookup.
        if shadowing_enabled() && custom_provider_registered(provider_id) {
            return Ok(ProviderType::Custom(provider_id.to_string()));
        }

        match provider_id {
            "anthropic" | "claude" => Ok(ProviderType::Claude),
            "openai" => Ok(ProviderType::OpenAI),
            "google" | "gemini" => Ok(ProviderType::Gemini),
            "zai" | "z-ai" => Ok(ProviderType::ZAI),
            "codex" => Ok(ProviderType::Codex),
            "github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot),
            other => {
                // PROV-067: Consult the custom provider registry before
                // failing. A custom provider slug maps 1:1 to
                // `ProviderType::Custom(slug)`.
                if custom_provider_registered(other) {
                    Ok(ProviderType::Custom(other.to_string()))
                } else {
                    Err(ProviderError::config(
                        "manager",
                        format!(
                            "Provider '{provider_id}' is not supported. Supported providers: anthropic, openai, google, zai, codex, github-copilot"
                        ),
                    ))
                }
            }
        }
    }

    /// PROV-101: A neutral placeholder `current_provider` for a registry-backed
    /// manager built via [`Self::with_model_support`]. It is ALWAYS overwritten
    /// by the explicit `select_model` / `set_model_direct` call that every
    /// caller makes next, so it is not a selection — just a value to hold the
    /// non-`Option` field until the explicit model arrives. Uses the first
    /// credentialed provider purely so accidental early reads stay credentialed;
    /// it never *chooses* among providers via a priority chain.
    fn deferred_placeholder_provider(credentials: &ProviderCredentials) -> ProviderType {
        if credentials.has_claude() {
            ProviderType::Claude
        } else if credentials.has_openai() {
            ProviderType::OpenAI
        } else if credentials.has_gemini() {
            ProviderType::Gemini
        } else if credentials.has_zai() {
            ProviderType::ZAI
        } else if credentials.has_codex() {
            ProviderType::Codex
        } else if credentials.has_github_copilot() {
            ProviderType::GitHubCopilot
        } else {
            ProviderType::OpenAI
        }
    }

    /// Get current provider name
    pub fn current_provider_name(&self) -> &str {
        self.current_provider.as_str()
    }

    /// PROV-067: Access the current [`ProviderType`] by reference. The
    /// `Custom(String)` variant is not `Copy`, so callers use this
    /// accessor instead of reading a public field.
    pub fn current_provider_type(&self) -> &ProviderType {
        &self.current_provider
    }

    /// PROV-067 / PROV-101: Test-only shim around the private
    /// [`crate::provider_resolution::resolve_unambiguous_provider`] helper, so integration tests
    /// can verify custom providers never auto-select even when they're the
    /// only credentialed provider, and that an ambiguous multi-provider state
    /// errors instead of silently picking Claude.
    #[doc(hidden)]
    pub fn detect_default_provider_for_test(
        credentials: &ProviderCredentials,
    ) -> Result<ProviderType, ProviderError> {
        crate::provider_resolution::resolve_unambiguous_provider(credentials)
    }

    /// Get Claude provider (if selected)
    ///
    /// MODEL-001: Now uses selected_model_id() for dynamic model selection.
    /// PROV-026: Checks claude_auth.json for OAuth tokens first (OAuth takes
    /// precedence over API key). Falls back to new_with_model() if no OAuth.
    pub fn get_claude(&self) -> Result<ClaudeProvider, ProviderError> {
        if self.current_provider == ProviderType::Claude {
            let model_id = self.selected_model_id();

            // PROV-026: Check claude_auth.json for OAuth tokens first
            if let Ok(Some(auth)) = crate::claude_auth::read_claude_auth_sync() {
                if !auth.access_token.is_empty() && !auth.refresh_token.is_empty() {
                    return ClaudeProvider::from_oauth_tokens(
                        &auth.access_token,
                        &auth.refresh_token,
                        Some(0), // Force immediate refresh — tokens from disk are of unknown age
                        crate::claude_oauth::CLAUDE_TOKEN_ENDPOINT_BASE,
                        model_id.as_deref().ok_or_else(|| {
                            ProviderError::config("claude", "Model is required. Please select a model before creating a session.")
                        })?,
                    );
                }
            }

            // Fall back to env var-based authentication
            ClaudeProvider::new_with_model(model_id.as_deref())
        } else {
            Err(ProviderError::config(
                "manager",
                "Current provider is not Claude",
            ))
        }
    }

    /// Get OpenAI provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    /// PROV-051: Accepts session_id for cache optimization (session affinity headers).
    pub fn get_openai(&self, session_id: uuid::Uuid) -> Result<OpenAIProvider, ProviderError> {
        if self.current_provider != ProviderType::OpenAI {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not OpenAI",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "openai",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| ProviderError::auth("openai", "OPENAI_API_KEY not set"))?;
        // PROV-165: the OpenAI request cap needs a known output limit.
        let max_output_tokens = self.max_output_tokens().ok_or_else(|| {
            ProviderError::config(
                "openai",
                format!("max output tokens unavailable for openai/{model_id}"),
            )
        })?;
        let max_output_tokens =
            std::num::NonZeroUsize::new(max_output_tokens).ok_or_else(|| {
                ProviderError::config(
                    "openai",
                    format!("max output tokens for openai/{model_id} is 0: set a positive limit"),
                )
            })?;
        Ok(
            OpenAIProvider::from_api_key_with_session(&api_key, &model_id, session_id)?
                .with_max_output_tokens(max_output_tokens),
        )
    }

    /// Get Codex provider (if selected)
    ///
    /// PROV-018: Sets CODEX_MODEL env var from the selected model so that
    /// CodexProvider::new() can read it. This bridges the TUI model selection
    /// (which stores the model in ProviderManager state) with the CodexProvider
    /// (which reads from the CODEX_MODEL env var).
    pub fn get_codex(&self) -> Result<CodexProvider, ProviderError> {
        if self.current_provider != ProviderType::Codex {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Codex",
            ));
        }

        // Set CODEX_MODEL from selected model for CodexProvider::new() to read
        if let Some(model_id) = self.selected_model_id() {
            std::env::set_var("CODEX_MODEL", &model_id);
        }

        let provider = CodexProvider::new()?.with_request_admission(
            crate::request_admission::AdmissionLimits {
                context_capacity: self
                    .raw_model_context_window()
                    .and_then(|capacity| u64::try_from(capacity).ok()),
                output_reservation: self
                    .raw_model_max_output_tokens()
                    .and_then(|reservation| u64::try_from(reservation).ok()),
            },
        );
        // MODEL-014: the request-option snapshot is the selected model's
        // Codex /models entry (advertised efforts and default effort); with
        // no usable entry there is none and nothing is invented (BUG-441).
        let snapshot = match (&self.codex_catalog, self.selected_model_id()) {
            (Some(Ok(catalog)), Some(model_id)) => catalog.snapshot(&model_id),
            _ => None,
        };
        provider
            .set_capabilities(snapshot)
            .map_err(|error| ProviderError::config("codex", error.to_string()))?;
        Ok(provider)
    }

    /// Get Gemini provider (if selected)
    ///
    /// Requires a model to be selected via select_model().
    pub fn get_gemini(&self) -> Result<GeminiProvider, ProviderError> {
        if self.current_provider != ProviderType::Gemini {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not Gemini",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "gemini",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let api_key = std::env::var("GOOGLE_GENERATIVE_AI_API_KEY")
            .map_err(|_| ProviderError::auth("gemini", "GOOGLE_GENERATIVE_AI_API_KEY not set"))?;
        GeminiProvider::from_api_key(&api_key, &model_id)
    }
}
