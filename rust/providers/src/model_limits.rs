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
/// * `default_context_window` / `default_max_output_tokens` — the limit the
///   provider declares for when no registry data and no user override exist,
///   or `None` when it declares none (PROV-165: the limit is then unavailable).
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

    /// Declared context window for when no registry data is available, or
    /// `None` when the provider declares none.
    fn default_context_window(&self) -> Option<usize>;

    /// Declared max output tokens for when no registry data is available, or
    /// `None` when the provider declares none.
    fn default_max_output_tokens(&self) -> Option<usize>;

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
/// 3. The provider's declared default
///
/// Returns `None` (unavailable) when none of the three exists.
/// # Arguments
///
/// * `registry_value` — value from an external registry (e.g. models.dev)
/// * `user_override`  — explicit user configuration
/// * `provider_max`   — the provider's hard ceiling (`None` = no ceiling)
/// * `provider_default` — the provider's declared default, if any
pub fn resolve_model_limits(
    registry_value: Option<usize>,
    user_override: Option<usize>,
    provider_max: Option<usize>,
    provider_default: Option<usize>,
) -> Option<usize> {
    // Helper: clamp `value` to `provider_max` when the provider declares one.
    let clamp = |value: usize| -> usize {
        match provider_max {
            Some(max) => value.min(max),
            None => value,
        }
    };

    user_override
        .or(registry_value)
        .map(clamp)
        .or(provider_default)
}

/// Convenience wrapper that resolves the context window for a provider.
///
/// Delegates to [`resolve_model_limits`] using the resolver's context-window
/// methods.
pub fn resolve_context_window(
    registry_value: Option<usize>,
    user_override: Option<usize>,
    resolver: &dyn ModelLimitsResolver,
) -> Option<usize> {
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
) -> Option<usize> {
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
mod tests;

// ---------------------------------------------------------------------------
// Provider-specific ModelLimitsResolver tests (LIMITS-003)
// ---------------------------------------------------------------------------

/// Feature: spec/features/provider-model-limits-resolution.feature
///
/// Tests that each real provider struct implements ModelLimitsResolver
/// with the correct hard limits, defaults, and send flags.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod provider_resolver_tests;

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
mod integration_all_providers;

/// Edge cases of the full resolution chain: zero values, missing registry
/// data and user-override priority (LIMITS-007).
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod integration_edge_cases;
