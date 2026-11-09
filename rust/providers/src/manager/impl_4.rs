impl ProviderManager {
    /// Get GitHub Copilot provider (if selected) — PROV-053 rule 9.
    ///
    /// Reads the persisted credential from `~/.fspec/credentials/copilot_auth.json`
    /// (written by `codelet auth login github-copilot`), determines the
    /// deployment type from the `enterprise_url` field, and constructs a
    /// [`CopilotProvider`] wired through the [`copilot::CopilotHttpClient`]
    /// middleware. The middleware injects the full Copilot header set
    /// (`x-initiator`, `User-Agent`, `Authorization`, `Openai-Intent`, and
    /// conditional `Copilot-Vision-Request`) on every outgoing request via
    /// the shared [`copilot::CopilotHeaderFacade`].
    ///
    /// Requires a model to be selected via `select_model()`.
    ///
    /// # Errors
    ///
    /// - [`ProviderError::Config`] if the current provider is not
    ///   `GitHubCopilot` or no model is selected
    /// - [`ProviderError::Auth`] if the credential file cannot be read or is
    ///   missing the access token
    pub fn get_github_copilot(&self) -> Result<CopilotProvider, ProviderError> {
        if self.current_provider != ProviderType::GitHubCopilot {
            return Err(ProviderError::config(
                "manager",
                "Current provider is not GitHub Copilot",
            ));
        }

        let model_id = self.selected_model_id().ok_or_else(|| {
            ProviderError::config(
                "github-copilot",
                "No model selected. Please select a model before starting a session.",
            )
        })?;

        let auth = copilot::read_copilot_auth_sync()
            .map_err(|e| {
                ProviderError::auth(
                    "github-copilot",
                    format!("Failed to read copilot_auth.json: {e}"),
                )
            })?
            .ok_or_else(|| {
                ProviderError::auth(
                    "github-copilot",
                    "No GitHub Copilot credential found. Run `codelet auth login github-copilot`.",
                )
            })?;

        if auth.github_oauth_token.is_empty() {
            return Err(ProviderError::auth(
                "github-copilot",
                "GitHub Copilot credential contains an empty access token. Re-run `codelet auth login github-copilot`.",
            ));
        }

        let deployment = match auth.enterprise_url.clone() {
            Some(host) => CopilotDeploymentType::Enterprise { host },
            None => CopilotDeploymentType::GitHubCom,
        };

        CopilotProvider::from_auth(deployment, auth, &model_id)
    }

    /// Check if any provider is available
    pub fn has_any_provider(&self) -> bool {
        self.credentials.has_any()
    }

    /// List all available providers for display
    pub fn list_available_providers(&self) -> Vec<String> {
        let mut providers = Vec::new();
        if self.credentials.has_claude() {
            providers.push("Claude (/claude)".to_string());
        }
        if self.credentials.has_openai() {
            providers.push("OpenAI (/openai)".to_string());
        }
        if self.credentials.has_gemini() {
            providers.push("Gemini (/gemini)".to_string());
        }
        if self.credentials.has_zai() {
            providers.push("Z.AI (/zai)".to_string());
        }
        if self.credentials.has_codex() {
            providers.push("Codex (/codex)".to_string());
        }
        if self.credentials.has_github_copilot() {
            providers.push("GitHub Copilot (/github-copilot)".to_string());
        }
        // PROV-067: Include discovered custom providers — both available
        // and unavailable — so users can see their full registry.
        let mut custom_names: Vec<&String> = self.credentials.custom_available.keys().collect();
        custom_names.sort();
        for name in custom_names {
            let available = self
                .credentials
                .custom_available
                .get(name)
                .copied()
                .unwrap_or(false);
            if available {
                providers.push(format!("{name} (/{name}) (custom)"));
            } else {
                providers.push(format!("{name} (/{name}) (custom, unavailable)"));
            }
        }
        providers
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<(), ProviderError> {
        let requested_provider = ProviderType::from_str(provider_name)?;

        // Validate requested provider has credentials
        if !requested_provider.has_credentials(&self.credentials) {
            return Err(ProviderError::auth(
                provider_name,
                format!("Provider {provider_name} not available. No credentials found."),
            ));
        }

        self.current_provider = requested_provider;
        self.selected_model = None;
        self.selected_registry_provider_id = None;
        self.selected_profile_name = None;
        self.user_context_window = None;
        self.user_max_output_tokens = None;
        self.facade_override = None;
        self.refresh_runtime_options()
    }

    /// Get prompt prefix for REPL (e.g., "[claude] > ")
    pub fn get_prompt_prefix(&self) -> String {
        format!("[{}] > ", self.current_provider.as_str())
    }

    /// LIMITS-004: Get the `ModelLimitsResolver` for the current provider.
    ///
    /// Returns a boxed resolver that declares the provider's hard limits and
    /// defaults. This avoids constructing a full provider (which requires
    /// credentials / API keys) — instead we build a lightweight resolver stub
    /// that mirrors the constants from each provider's `ModelLimitsResolver`
    /// impl.
    fn provider_limits_resolver(&self) -> Box<dyn ModelLimitsResolver> {
        match &self.current_provider {
            ProviderType::Claude => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: Some(claude::CONTEXT_WINDOW),
                max_out: None,
                default_out: Some(claude::MAX_OUTPUT_TOKENS),
            }),
            // PROV-171: OpenAI trusts the registry; its only defaults are the
            // OPENAI_* overrides captured at selection — none means unavailable.
            ProviderType::OpenAI => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: self.openai_env_limits.context_window,
                max_out: None,
                default_out: self.openai_env_limits.max_output_tokens,
            }),
            ProviderType::Gemini => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: Some(gemini::CONTEXT_WINDOW),
                max_out: None,
                default_out: Some(gemini::MAX_OUTPUT_TOKENS),
            }),
            // PROV-165: Codex declares no limits; they come from the
            // catalog or the user, or are unavailable.
            ProviderType::Codex => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: None,
                max_out: None,
                default_out: None,
            }),
            ProviderType::ZAI => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: Some(zai::CONTEXT_WINDOW),
                max_out: None,
                default_out: Some(zai::MAX_OUTPUT_TOKENS),
            }),
            ProviderType::GitHubCopilot => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: Some(copilot::CONTEXT_WINDOW),
                max_out: None,
                default_out: Some(copilot::MAX_OUTPUT_TOKENS),
            }),
            // BUG-448: the test-only stub provider declares its own limits.
            #[cfg(feature = "test-support")]
            ProviderType::Custom(name) if name == "stub" => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: Some(crate::stub_provider::STUB_CONTEXT_WINDOW),
                max_out: None,
                default_out: Some(crate::stub_provider::STUB_MAX_OUTPUT_TOKENS),
            }),
            // PROV-067: Custom providers have no hard ceiling — trust
            // user/registry values. PROV-165: they declare no defaults, so
            // without those values the limits are unavailable (no guessed
            // 128k / 4k).
            ProviderType::Custom(_) => Box::new(ConstantResolver {
                max_ctx: None,
                default_ctx: None,
                max_out: None,
                default_out: None,
            }),
        }
    }

    /// Get context window size for the current provider.
    ///
    /// LIMITS-004: Resolves through `ModelLimitsResolver` so that provider
    /// hard limits clamp registry and user-override values.
    ///
    /// Priority chain (highest → lowest):
    /// 1. User override — clamped by provider max
    /// 2. Registry value — clamped by provider max
    /// 3. Provider default
    ///
    /// PROV-165: `None` when none of them exists (the limit is unavailable).
    /// BUG-547: a context window the profile's engine reported for this
    /// selection is used only when nothing is declared.
    pub fn context_window(&self) -> Option<usize> {
        let resolver = self.provider_limits_resolver();
        resolve_context_window(
            self.registry_context_window,
            self.user_context_window
                .or(self.runtime_options.context_window)
                .or(self.engine_context_window().map(|limit| limit.tokens)),
            resolver.as_ref(),
        )
    }

    /// Get max output tokens for the current provider.
    ///
    /// LIMITS-004: Resolves through `ModelLimitsResolver` so that provider
    /// hard limits clamp registry and user-override values.
    ///
    /// PROV-165: `None` when the limit is unavailable.
    pub fn max_output_tokens(&self) -> Option<usize> {
        let resolver = self.provider_limits_resolver();
        resolve_max_output_tokens(
            self.registry_max_output_tokens,
            self.user_max_output_tokens
                .or(self.runtime_options.max_output_tokens),
            resolver.as_ref(),
        )
    }

    /// LIMITS-004: Override the per-model context window and max output tokens.
    ///
    /// Used by the NAPI layer to apply TypeScript overrides on top of
    /// models.dev registry data. Stored as user overrides so the resolver
    /// can clamp them by the provider's hard maximum.
    pub fn override_model_limits(
        &mut self,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) {
        if let Some(cw) = context_window {
            self.user_context_window = Some(cw);
        }
        if let Some(mot) = max_output_tokens {
            self.user_max_output_tokens = Some(mot);
        }
    }
}
