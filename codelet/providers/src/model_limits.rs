//! Model Limits Resolution (LIMITS-002)
//!
//! Defines the `ModelLimitsResolver` trait and the `resolve_model_limits` pure
//! function that implements the priority chain:
//!
//! ```text
//! user_override → clamp by provider max → registry_value → clamp by provider max → provider default
//! ```
//!
//! Providers implement `ModelLimitsResolver` to declare hard ceilings and
//! defaults for context window and max output tokens.  The standalone
//! `resolve_model_limits` function is intentionally *not* a method on
//! `ProviderManager` so it can be tested in isolation.

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Providers implement this trait to declare their hard API limits.
///
/// * `max_context_window` / `max_output_tokens_limit` — return `Some(n)` to
///   clamp registry / user values, or `None` to trust them as-is.
/// * `default_context_window` / `default_max_output_tokens` — fallback when
///   no registry data and no user override exist.
/// * `should_send_max_output_tokens` — set to `false` for providers whose API
///   rejects the `max_output_tokens` parameter (e.g. Codex).
pub trait ModelLimitsResolver: Send + Sync {
    /// The provider's hard maximum context window.
    ///
    /// Registry and user-override values will be clamped to this ceiling.
    /// Return `None` to trust external values without clamping.
    fn max_context_window(&self) -> Option<usize> {
        None
    }

    /// The provider's hard maximum output tokens.
    ///
    /// Registry and user-override values will be clamped to this ceiling.
    /// Return `None` to trust external values without clamping.
    fn max_output_tokens_limit(&self) -> Option<usize> {
        None
    }

    /// Fallback context window when no registry data is available.
    fn default_context_window(&self) -> usize;

    /// Fallback max output tokens when no registry data is available.
    fn default_max_output_tokens(&self) -> usize;

    /// Whether the `max_output_tokens` parameter should be sent in API
    /// requests.
    ///
    /// Returns `true` by default.  Providers whose API rejects this field
    /// (e.g. Codex) should override to return `false`.
    fn should_send_max_output_tokens(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Resolution function
// ---------------------------------------------------------------------------

/// Resolve a single model limit value using the priority chain.
///
/// Priority (highest → lowest):
/// 1. `user_override` — clamped by provider max when present
/// 2. `registry_value` — clamped by provider max when present
/// 3. Provider default (via `default_fn`)
///
/// # Arguments
///
/// * `registry_value` — value from an external registry (e.g. models.dev)
/// * `user_override`  — explicit user configuration
/// * `provider_max`   — the provider's hard ceiling (`None` = no ceiling)
/// * `provider_default` — fallback value from the provider
pub fn resolve_model_limits(
    registry_value: Option<usize>,
    user_override: Option<usize>,
    provider_max: Option<usize>,
    provider_default: usize,
) -> usize {
    // Helper: clamp `value` to `provider_max` when the provider declares one.
    let clamp = |value: usize| -> usize {
        match provider_max {
            Some(max) => value.min(max),
            None => value,
        }
    };

    if let Some(user) = user_override {
        return clamp(user);
    }

    if let Some(registry) = registry_value {
        return clamp(registry);
    }

    provider_default
}

/// Convenience wrapper that resolves the context window for a provider.
///
/// Delegates to [`resolve_model_limits`] using the resolver's context-window
/// methods.
pub fn resolve_context_window(
    registry_value: Option<usize>,
    user_override: Option<usize>,
    resolver: &dyn ModelLimitsResolver,
) -> usize {
    resolve_model_limits(
        registry_value,
        user_override,
        resolver.max_context_window(),
        resolver.default_context_window(),
    )
}

/// Convenience wrapper that resolves max output tokens for a provider.
///
/// Delegates to [`resolve_model_limits`] using the resolver's output-token
/// methods.
pub fn resolve_max_output_tokens(
    registry_value: Option<usize>,
    user_override: Option<usize>,
    resolver: &dyn ModelLimitsResolver,
) -> usize {
    resolve_model_limits(
        registry_value,
        user_override,
        resolver.max_output_tokens_limit(),
        resolver.default_max_output_tokens(),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Feature: spec/features/modellimitsresolver-trait-provider-veto-authority.feature
///
/// This test module validates the acceptance criteria defined in the feature
/// file.  Scenarios map directly to Gherkin scenarios.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // -- Test doubles -------------------------------------------------------

    /// A Claude-like resolver that clamps to 200 000.
    struct ClampingResolver {
        max_ctx: Option<usize>,
        default_ctx: usize,
        max_out: Option<usize>,
        default_out: usize,
        send_max_output: bool,
    }

    impl ClampingResolver {
        fn new() -> Self {
            Self {
                max_ctx: Some(200_000),
                default_ctx: 200_000,
                max_out: Some(8_192),
                default_out: 8_192,
                send_max_output: true,
            }
        }

        fn trusting() -> Self {
            Self {
                max_ctx: None,
                default_ctx: 128_000,
                max_out: None,
                default_out: 4_096,
                send_max_output: true,
            }
        }

        fn codex_like() -> Self {
            Self {
                max_ctx: None,
                default_ctx: 272_000,
                max_out: None,
                default_out: 4_096,
                send_max_output: false,
            }
        }
    }

    impl ModelLimitsResolver for ClampingResolver {
        fn max_context_window(&self) -> Option<usize> {
            self.max_ctx
        }

        fn max_output_tokens_limit(&self) -> Option<usize> {
            self.max_out
        }

        fn default_context_window(&self) -> usize {
            self.default_ctx
        }

        fn default_max_output_tokens(&self) -> usize {
            self.default_out
        }

        fn should_send_max_output_tokens(&self) -> bool {
            self.send_max_output
        }
    }

    // -- Scenario tests -----------------------------------------------------

    #[test]
    fn provider_clamps_registry_value_to_its_hard_maximum() {
        // @step Given a provider declares max_context_window as 200000
        let resolver = ClampingResolver::new();

        // @step And the registry reports a context window of 1000000
        let registry_value = Some(1_000_000);

        // @step And no user override is set
        let user_override = None;

        // @step When the model limits are resolved
        let result = resolve_context_window(registry_value, user_override, &resolver);

        // @step Then the resolved context window should be 200000
        assert_eq!(result, 200_000);
    }

    #[test]
    fn provider_trusts_registry_value_when_no_max_is_declared() {
        // @step Given a provider declares max_context_window as None
        let resolver = ClampingResolver::trusting();

        // @step And the registry reports a context window of 128000
        let registry_value = Some(128_000);

        // @step And no user override is set
        let user_override = None;

        // @step When the model limits are resolved
        let result = resolve_context_window(registry_value, user_override, &resolver);

        // @step Then the resolved context window should be 128000
        assert_eq!(result, 128_000);
    }

    #[test]
    fn user_override_is_clamped_by_provider_max() {
        // @step Given a provider declares max_context_window as 200000
        let resolver = ClampingResolver::new();

        // @step And no registry value is available
        let registry_value = None;

        // @step And the user override is set to 500000
        let user_override = Some(500_000);

        // @step When the model limits are resolved
        let result = resolve_context_window(registry_value, user_override, &resolver);

        // @step Then the resolved context window should be 200000
        assert_eq!(result, 200_000);
    }

    #[test]
    fn user_override_is_trusted_when_provider_declares_no_max() {
        // @step Given a provider declares max_context_window as None
        let resolver = ClampingResolver::trusting();

        // @step And no registry value is available
        let registry_value = None;

        // @step And the user override is set to 100000
        let user_override = Some(100_000);

        // @step When the model limits are resolved
        let result = resolve_context_window(registry_value, user_override, &resolver);

        // @step Then the resolved context window should be 100000
        assert_eq!(result, 100_000);
    }

    #[test]
    fn provider_default_is_used_when_no_registry_or_user_override_exists() {
        // @step Given a provider declares default_context_window as 272000
        let resolver = ClampingResolver::codex_like();

        // @step And no registry value is available
        let registry_value = None;

        // @step And no user override is set
        let user_override = None;

        // @step When the model limits are resolved
        let result = resolve_context_window(registry_value, user_override, &resolver);

        // @step Then the resolved context window should be 272000
        assert_eq!(result, 272_000);
    }

    #[test]
    fn provider_can_suppress_sending_max_output_tokens() {
        // @step Given a provider declares should_send_max_output_tokens as false
        let resolver = ClampingResolver::codex_like();

        // @step Then the resolver should indicate max_output_tokens must not be sent
        assert!(!resolver.should_send_max_output_tokens());
    }

    #[test]
    fn max_output_tokens_are_clamped_by_provider_limit() {
        // @step Given a provider declares max_output_tokens_limit as 8192
        let resolver = ClampingResolver::new();

        // @step And the registry reports max output tokens of 128000
        let registry_value = Some(128_000);

        // @step And no user override is set
        let user_override = None;

        // @step When the model output limits are resolved
        let result = resolve_max_output_tokens(registry_value, user_override, &resolver);

        // @step Then the resolved max output tokens should be 8192
        assert_eq!(result, 8_192);
    }

    #[test]
    fn provider_default_max_output_tokens_used_when_no_registry_data() {
        // @step Given a provider declares default_max_output_tokens as 4096
        let resolver = ClampingResolver::codex_like();

        // @step And no registry value is available
        let registry_value = None;

        // @step And no user override is set
        let user_override = None;

        // @step When the model output limits are resolved
        let result = resolve_max_output_tokens(registry_value, user_override, &resolver);

        // @step Then the resolved max output tokens should be 4096
        assert_eq!(result, 4_096);
    }

    // -- Default trait method tests -----------------------------------------

    #[test]
    fn should_send_max_output_tokens_defaults_to_true() {
        struct MinimalResolver;
        impl ModelLimitsResolver for MinimalResolver {
            fn default_context_window(&self) -> usize {
                128_000
            }
            fn default_max_output_tokens(&self) -> usize {
                4_096
            }
        }
        let resolver = MinimalResolver;
        assert!(resolver.should_send_max_output_tokens());
    }

    #[test]
    fn max_context_window_defaults_to_none() {
        struct MinimalResolver;
        impl ModelLimitsResolver for MinimalResolver {
            fn default_context_window(&self) -> usize {
                128_000
            }
            fn default_max_output_tokens(&self) -> usize {
                4_096
            }
        }
        let resolver = MinimalResolver;
        assert_eq!(resolver.max_context_window(), None);
    }

    #[test]
    fn max_output_tokens_limit_defaults_to_none() {
        struct MinimalResolver;
        impl ModelLimitsResolver for MinimalResolver {
            fn default_context_window(&self) -> usize {
                128_000
            }
            fn default_max_output_tokens(&self) -> usize {
                4_096
            }
        }
        let resolver = MinimalResolver;
        assert_eq!(resolver.max_output_tokens_limit(), None);
    }
}

// ---------------------------------------------------------------------------
// Provider-specific ModelLimitsResolver tests (LIMITS-003)
// ---------------------------------------------------------------------------

/// Feature: spec/features/provider-model-limits-resolution.feature
///
/// Tests that each real provider struct implements ModelLimitsResolver
/// with the correct hard limits, defaults, and send flags.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod provider_resolver_tests {
    use super::*;

    // -- Claude Provider Tests ----------------------------------------------

    /// Scenario: Claude resolver clamps registry context window to 200k
    #[test]
    fn claude_resolver_clamps_registry_context_window_to_200k() {
        // @step Given the Claude provider's resolver declares max_context_window as 200000
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), Some(200_000));

        // @step When the registry reports a context window of 1000000
        let registry_value = Some(1_000_000);

        // @step Then the resolved context window should be clamped to 200000
        let result = resolve_context_window(registry_value, None, &resolver);
        assert_eq!(result, 200_000);
    }

    /// Scenario: Claude resolver clamps registry max output tokens to 8192
    #[test]
    fn claude_resolver_clamps_registry_max_output_tokens_to_8192() {
        // @step Given the Claude provider's resolver declares max_output_tokens_limit as 8192
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_output_tokens_limit(), Some(8_192));

        // @step When the registry reports max output tokens of 128000
        let registry_value = Some(128_000);

        // @step Then the resolved max output tokens should be clamped to 8192
        let result = resolve_max_output_tokens(registry_value, None, &resolver);
        assert_eq!(result, 8_192);
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
        assert_eq!(resolver.default_context_window(), 200_000);

        // @step And the default max output tokens should be 8192
        assert_eq!(resolver.default_max_output_tokens(), 8_192);

        // @step And should_send_max_output_tokens should be true
        assert!(resolver.should_send_max_output_tokens());
    }

    // -- OpenAI Provider Tests ----------------------------------------------

    /// Scenario: OpenAI resolver trusts registry values
    #[test]
    fn openai_resolver_trusts_registry_values() {
        // @step Given the OpenAI provider's resolver declares max_context_window as None
        let resolver = crate::openai::OpenAIProvider::from_api_key_with_options(
            "test-key",
            "gpt-4o",
            None,
            None,
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), None);
        assert_eq!(resolver.max_output_tokens_limit(), None);

        // @step When the registry reports a context window of 128000 and max output tokens of 16384
        let ctx_result = resolve_context_window(Some(128_000), None, &resolver);
        let out_result = resolve_max_output_tokens(Some(16_384), None, &resolver);

        // @step Then the resolved context window should be 128000
        assert_eq!(ctx_result, 128_000);

        // @step And the resolved max output tokens should be 16384
        assert_eq!(out_result, 16_384);
    }

    /// Scenario: OpenAI resolver reads default from OPENAI_CONTEXT_WINDOW env var
    #[test]
    #[serial_test::serial]
    fn openai_resolver_reads_context_window_env_var() {
        // @step Given the OPENAI_CONTEXT_WINDOW environment variable is set to 256000
        std::env::set_var("OPENAI_CONTEXT_WINDOW", "256000");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        // @step When the OpenAI resolver is queried with no registry data
        let resolver = crate::openai::OpenAIProvider::from_api_key_with_options(
            "test-key",
            "gpt-4o",
            None,
            None,
        )
        .expect("Should create provider");

        // @step Then the default context window should be 256000
        assert_eq!(resolver.default_context_window(), 256_000);

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
        let resolver = crate::openai::OpenAIProvider::from_api_key_with_options(
            "test-key",
            "gpt-4o",
            None,
            None,
        )
        .expect("Should create provider");

        // @step Then the default max output tokens should be 16384
        assert_eq!(resolver.default_max_output_tokens(), 16_384);

        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    }

    /// Scenario: OpenAI resolver falls back to compile-time defaults
    #[test]
    #[serial_test::serial]
    fn openai_resolver_falls_back_to_compile_time_defaults() {
        // @step Given no OPENAI_CONTEXT_WINDOW or OPENAI_MAX_OUTPUT_TOKENS env vars are set
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        // @step When the OpenAI resolver is queried with no registry data
        let resolver = crate::openai::OpenAIProvider::from_api_key_with_options(
            "test-key",
            "gpt-4o",
            None,
            None,
        )
        .expect("Should create provider");

        // @step Then the default context window should be 128000
        assert_eq!(resolver.default_context_window(), 128_000);

        // @step And the default max output tokens should be 4096
        assert_eq!(resolver.default_max_output_tokens(), 4_096);

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
        assert_eq!(resolver.default_context_window(), 1_000_000);

        // @step And the default max output tokens should be 8192
        assert_eq!(resolver.default_max_output_tokens(), 8_192);

        // @step And should_send_max_output_tokens should be true
        assert!(resolver.should_send_max_output_tokens());
    }

    // -- Codex Provider Tests -----------------------------------------------

    /// Scenario: Codex resolver returns correct defaults and suppresses max_output_tokens
    #[test]
    fn codex_resolver_returns_correct_defaults_and_suppresses_max_output_tokens() {
        // @step Given the Codex provider's resolver is queried
        let resolver =
            crate::codex::CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
                .expect("Should create provider");

        // @step Then the default context window should be 272000
        assert_eq!(resolver.default_context_window(), 272_000);

        // @step And the default max output tokens should be 4096
        assert_eq!(resolver.default_max_output_tokens(), 4_096);

        // @step And should_send_max_output_tokens should be false
        assert!(!resolver.should_send_max_output_tokens());
    }

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
        assert_eq!(result, 300_000);
    }

    // -- Z.AI Provider Tests ------------------------------------------------

    /// Scenario: Z.AI resolver returns correct defaults
    #[test]
    fn zai_resolver_returns_correct_defaults() {
        // @step Given the Z.AI provider's resolver is queried
        let resolver = crate::zai::ZAIProvider::from_api_key("test-key", "glm-4.7")
            .expect("Should create provider");

        // @step Then the default context window should be 128000
        assert_eq!(resolver.default_context_window(), 128_000);

        // @step And the default max output tokens should be 8192
        assert_eq!(resolver.default_max_output_tokens(), 8_192);

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
        assert_eq!(resolver.default_context_window(), 200_000);

        // @step And the default max output tokens should be 4096
        assert_eq!(resolver.default_max_output_tokens(), 4_096);

        // @step And should_send_max_output_tokens should be true
        assert!(resolver.should_send_max_output_tokens());
    }
}

// ---------------------------------------------------------------------------
// LIMITS-007: Integration Tests — All Provider/Model Combinations
// ---------------------------------------------------------------------------

/// Feature: spec/features/integration-tests-all-provider-model-combinations.feature
///
/// Comprehensive integration tests verifying the FULL model limits resolution
/// chain for every provider/model combination. Tests the resolve_model_limits
/// pure function and ProviderManager.context_window() / max_output_tokens()
/// through ModelLimitsResolver for all 6 providers.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod integration_all_providers {
    use super::*;
    use crate::manager::{ProviderManager, ProviderType};

    // =========================================================================
    // Scenario: Claude Sonnet 4 resolves context window to 200k and max output to 8192
    // =========================================================================

    #[test]
    fn claude_sonnet_4_resolves_context_200k_output_8192() {
        // @step Given the Claude provider resolver with max_context_window 200000 and max_output_tokens_limit 8192
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), Some(200_000));
        assert_eq!(resolver.max_output_tokens_limit(), Some(8_192));

        // @step When resolve_context_window is called with registry value 200000 and no user override
        let ctx = resolve_context_window(Some(200_000), None, &resolver);
        let out = resolve_max_output_tokens(Some(8_192), None, &resolver);

        // @step Then context_window should be 200000 and max_output_tokens should be 8192
        assert_eq!(ctx, 200_000);
        assert_eq!(out, 8_192);
    }

    // =========================================================================
    // Scenario: Claude Opus 4.6 clamps 1M registry to 200k and 128k output to 8192
    // =========================================================================

    #[test]
    fn claude_opus_4_6_clamps_1m_to_200k_and_128k_to_8192() {
        // @step Given the Claude provider resolver with max_context_window 200000 and max_output_tokens_limit 8192
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-opus-4-6-20250610",
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), Some(200_000));
        assert_eq!(resolver.max_output_tokens_limit(), Some(8_192));

        // @step When resolve_context_window is called with registry value 1000000 and no user override
        let ctx = resolve_context_window(Some(1_000_000), None, &resolver);
        let out = resolve_max_output_tokens(Some(128_000), None, &resolver);

        // @step Then context_window should be 200000 and max_output_tokens should be 8192
        assert_eq!(ctx, 200_000, "Claude must clamp 1M registry to 200k");
        assert_eq!(out, 8_192, "Claude must clamp 128k output to 8192");
    }

    // =========================================================================
    // Scenario: OpenAI gpt-4o trusts registry values without clamping
    // =========================================================================

    #[test]
    #[serial_test::serial]
    fn openai_gpt4o_trusts_registry_values() {
        // @step Given the OpenAI provider resolver with no max_context_window and default 128000
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        let resolver = crate::openai::OpenAIProvider::from_api_key_with_options(
            "test-key",
            "gpt-4o",
            None,
            None,
        )
        .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), None);
        assert_eq!(resolver.max_output_tokens_limit(), None);

        // @step When resolve_context_window is called with registry value 128000
        let ctx = resolve_context_window(Some(128_000), None, &resolver);
        let out = resolve_max_output_tokens(Some(16_384), None, &resolver);

        // @step Then context_window should be 128000 and max_output_tokens should be 16384
        assert_eq!(ctx, 128_000, "OpenAI should pass through 128k unchanged");
        assert_eq!(out, 16_384, "OpenAI should pass through 16384 unchanged");
    }

    // =========================================================================
    // Scenario: Gemini 2.5 Pro trusts 1M registry value and uses 80% threshold
    // =========================================================================

    #[test]
    fn gemini_2_5_pro_trusts_1m_registry() {
        // @step Given the Gemini provider resolver with no max_context_window and default 1000000
        let resolver = crate::gemini::GeminiProvider::from_api_key("test-key", "gemini-2.5-pro")
            .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), None);
        assert_eq!(resolver.default_context_window(), 1_000_000);

        // @step When resolve_context_window is called with registry value 1000000
        let ctx = resolve_context_window(Some(1_000_000), None, &resolver);
        let out = resolve_max_output_tokens(Some(8_192), None, &resolver);

        // @step Then context_window should be 1000000 and max_output_tokens should be 8192
        assert_eq!(ctx, 1_000_000, "Gemini should trust 1M registry value");
        assert_eq!(out, 8_192, "Gemini should trust 8192 output tokens");
    }

    // =========================================================================
    // Scenario: Codex falls back to 272k default context window with no registry
    // =========================================================================

    #[test]
    fn codex_falls_back_to_272k_default() {
        // @step Given the Codex provider resolver with no max_context_window and default 272000
        let resolver =
            crate::codex::CodexProvider::from_api_key("sk-proj-test-key-12345", "gpt-5.1-codex")
                .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), None);
        assert_eq!(resolver.default_context_window(), 272_000);

        // @step When resolve_context_window is called with no registry value and no user override
        let ctx = resolve_context_window(None, None, &resolver);
        let out = resolve_max_output_tokens(None, None, &resolver);

        // @step Then context_window should be 272000 and max_output_tokens should be 4096
        assert_eq!(ctx, 272_000, "Codex should use 272k default");
        assert_eq!(out, 4_096, "Codex should use 4096 default");
        assert!(!resolver.should_send_max_output_tokens(), "Codex suppresses max_output_tokens");
    }

    // =========================================================================
    // Scenario: Z.AI resolves 128k context window and 8192 max output
    // =========================================================================

    #[test]
    fn zai_resolves_128k_context_8192_output() {
        // @step Given the Z.AI provider resolver with no max_context_window and default 128000
        let resolver = crate::zai::ZAIProvider::from_api_key("test-key", "glm-4-plus")
            .expect("Should create provider");

        assert_eq!(resolver.max_context_window(), None);
        assert_eq!(resolver.default_context_window(), 128_000);

        // @step When resolve_context_window is called with no registry value
        let ctx = resolve_context_window(None, None, &resolver);
        let out = resolve_max_output_tokens(None, None, &resolver);

        // @step Then context_window should be 128000 and max_output_tokens should be 8192
        assert_eq!(ctx, 128_000, "Z.AI should use 128k default");
        assert_eq!(out, 8_192, "Z.AI should use 8192 default");
    }

    // =========================================================================
    // Scenario: Copilot falls back to 200k default context and 4096 max output
    // =========================================================================

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
        assert_eq!(resolver.default_context_window(), 200_000);

        // @step When resolve_context_window is called with no registry value
        let ctx = resolve_context_window(None, None, &resolver);
        let out = resolve_max_output_tokens(None, None, &resolver);

        // @step Then context_window should be 200000 and max_output_tokens should be 4096
        assert_eq!(ctx, 200_000, "Copilot should use 200k default");
        assert_eq!(out, 4_096, "Copilot should use 4096 default");
    }

    // =========================================================================
    // Scenario: User override exceeding provider max is clamped
    // =========================================================================

    #[test]
    fn user_override_exceeding_provider_max_is_clamped() {
        // @step Given the Claude provider resolver with max_context_window 200000
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        // @step When a user override of 500000 is applied
        let ctx = resolve_context_window(None, Some(500_000), &resolver);
        let out = resolve_max_output_tokens(None, Some(32_000), &resolver);

        // @step Then context_window should be clamped to 200000
        assert_eq!(ctx, 200_000, "User override 500k must be clamped to Claude max 200k");
        assert_eq!(out, 8_192, "User override 32k must be clamped to Claude max 8192");
    }

    // =========================================================================
    // Scenario: ProviderManager full chain resolves clamped values for all providers
    // =========================================================================

    #[test]
    #[serial_test::serial]
    fn provider_manager_full_chain_all_providers() {
        // @step Given a ProviderManager configured for each provider type
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        // Claude with 1M registry → clamped to 200k
        // @step When context_window() and max_output_tokens() are called with registry values
        let claude = ProviderManager::for_testing(
            ProviderType::Claude,
            Some(1_000_000),
            Some(128_000),
        );
        // @step Then each provider returns correctly clamped or pass-through values
        assert_eq!(claude.context_window(), 200_000, "Claude: 1M clamped to 200k");
        assert_eq!(claude.max_output_tokens(), 8_192, "Claude: 128k clamped to 8192");

        // OpenAI with registry values → pass-through
        let openai = ProviderManager::for_testing(
            ProviderType::OpenAI,
            Some(128_000),
            Some(16_384),
        );
        assert_eq!(openai.context_window(), 128_000, "OpenAI: 128k pass-through");
        assert_eq!(openai.max_output_tokens(), 16_384, "OpenAI: 16384 pass-through");

        // Gemini with 1M registry → pass-through
        let gemini = ProviderManager::for_testing(
            ProviderType::Gemini,
            Some(1_000_000),
            Some(8_192),
        );
        assert_eq!(gemini.context_window(), 1_000_000, "Gemini: 1M pass-through");
        assert_eq!(gemini.max_output_tokens(), 8_192, "Gemini: 8192 pass-through");

        // Codex no registry → defaults
        let codex = ProviderManager::for_testing(
            ProviderType::Codex,
            None,
            None,
        );
        assert_eq!(codex.context_window(), 272_000, "Codex: 272k default");
        assert_eq!(codex.max_output_tokens(), 4_096, "Codex: 4096 default");

        // Z.AI no registry → defaults
        let zai = ProviderManager::for_testing(
            ProviderType::ZAI,
            None,
            None,
        );
        assert_eq!(zai.context_window(), 128_000, "Z.AI: 128k default");
        assert_eq!(zai.max_output_tokens(), 8_192, "Z.AI: 8192 default");

        // Copilot no registry → defaults
        let copilot = ProviderManager::for_testing(
            ProviderType::GitHubCopilot,
            None,
            None,
        );
        assert_eq!(copilot.context_window(), 200_000, "Copilot: 200k default");
        assert_eq!(copilot.max_output_tokens(), 4_096, "Copilot: 4096 default");
    }

    // =========================================================================
    // Scenario: Sub-agent propagation returns clamped values for Claude
    // =========================================================================

    #[test]
    fn sub_agent_propagation_returns_clamped_claude() {
        // @step Given a ProviderManager with Claude and registry context_window of 1000000
        let mut manager = ProviderManager::for_testing(
            ProviderType::Claude,
            Some(1_000_000),
            Some(128_000),
        );
        // Ensure registry values are set
        manager.registry_context_window = Some(1_000_000);
        manager.registry_max_output_tokens = Some(128_000);

        // @step When raw_model_context_window() is called for sub-agent propagation
        let raw_ctx = manager.raw_model_context_window();
        let raw_out = manager.raw_model_max_output_tokens();

        // @step Then the returned value should be 200000 not 1000000
        assert_eq!(raw_ctx, Some(200_000), "Sub-agent gets clamped 200k, not raw 1M");
        assert_eq!(raw_out, Some(8_192), "Sub-agent gets clamped 8192, not raw 128k");
    }

    // =========================================================================
    // Additional edge cases: zero values, missing registry, user override priority
    // =========================================================================

    /// Edge case: Zero user override on a trusting provider → returns 0
    #[test]
    fn zero_user_override_on_trusting_provider_returns_zero() {
        let resolver = crate::gemini::GeminiProvider::from_api_key("test-key", "gemini-2.0-flash")
            .expect("Should create provider");

        // User explicitly sets 0 — no clamping since provider has no max
        let ctx = resolve_context_window(None, Some(0), &resolver);
        assert_eq!(ctx, 0, "Zero user override passes through unclamped on trusting provider");
    }

    /// Edge case: User override below provider max → not clamped
    #[test]
    fn user_override_below_provider_max_not_clamped() {
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        let ctx = resolve_context_window(None, Some(100_000), &resolver);
        assert_eq!(ctx, 100_000, "User override 100k is below Claude max 200k, should not clamp");
    }

    /// Edge case: Registry and user override both present — user wins
    #[test]
    fn user_override_takes_priority_over_registry() {
        let resolver = crate::claude::ClaudeProvider::from_api_key_with_model(
            "sk-ant-api03-test-key",
            "claude-sonnet-4-20250514",
        )
        .expect("Should create provider");

        // Registry says 200k, user says 150k
        let ctx = resolve_context_window(Some(200_000), Some(150_000), &resolver);
        assert_eq!(ctx, 150_000, "User override 150k wins over registry 200k");
    }

    /// Edge case: ProviderManager override_model_limits stores and clamps correctly
    #[test]
    fn provider_manager_override_model_limits_clamps() {
        let mut manager = ProviderManager::for_testing(
            ProviderType::Claude,
            Some(200_000),
            Some(8_192),
        );

        // Override with values exceeding provider max
        manager.override_model_limits(Some(500_000), Some(64_000));

        // Both should be clamped to Claude's hard max
        assert_eq!(manager.context_window(), 200_000, "Override 500k clamped to 200k");
        assert_eq!(manager.max_output_tokens(), 8_192, "Override 64k clamped to 8192");
    }

    /// Edge case: ProviderManager raw_model_context_window returns None when no data
    #[test]
    fn provider_manager_raw_returns_none_when_no_data() {
        let manager = ProviderManager::for_testing(
            ProviderType::OpenAI,
            None,
            None,
        );

        assert_eq!(manager.raw_model_context_window(), None, "No data → None for sub-agent");
        assert_eq!(manager.raw_model_max_output_tokens(), None, "No data → None for sub-agent");
    }

    /// Copilot with registry values → pass-through (not clamped like Claude)
    #[test]
    fn copilot_with_registry_values_passes_through() {
        let copilot = ProviderManager::for_testing(
            ProviderType::GitHubCopilot,
            Some(128_000),
            Some(16_384),
        );
        assert_eq!(copilot.context_window(), 128_000, "Copilot passes through registry 128k");
        assert_eq!(copilot.max_output_tokens(), 16_384, "Copilot passes through registry 16384");
    }

    /// Z.AI with registry values → pass-through
    #[test]
    fn zai_with_registry_values_passes_through() {
        let zai = ProviderManager::for_testing(
            ProviderType::ZAI,
            Some(128_000),
            Some(8_192),
        );
        assert_eq!(zai.context_window(), 128_000, "Z.AI passes through registry 128k");
        assert_eq!(zai.max_output_tokens(), 8_192, "Z.AI passes through registry 8192");
    }

    /// Codex with registry values → pass-through (no clamping)
    #[test]
    fn codex_with_registry_values_passes_through() {
        let codex = ProviderManager::for_testing(
            ProviderType::Codex,
            Some(300_000),
            Some(8_192),
        );
        assert_eq!(codex.context_window(), 300_000, "Codex passes through registry 300k");
        assert_eq!(codex.max_output_tokens(), 8_192, "Codex passes through registry 8192");
    }

    /// OpenAI o3 with large registry values → pass-through (trusts registry)
    #[test]
    #[serial_test::serial]
    fn openai_o3_with_large_registry_passes_through() {
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

        let openai = ProviderManager::for_testing(
            ProviderType::OpenAI,
            Some(200_000),
            Some(100_000),
        );
        assert_eq!(openai.context_window(), 200_000, "OpenAI o3 passes through 200k");
        assert_eq!(openai.max_output_tokens(), 100_000, "OpenAI o3 passes through 100k");
    }
}
