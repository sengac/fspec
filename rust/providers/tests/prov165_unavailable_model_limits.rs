//! PROV-165: unavailable model limits are a typed state, not 0.
//!
//! Feature: spec/features/unavailable-model-limits.feature

use codelet_providers::model_limits::{resolve_context_window, resolve_max_output_tokens};
use codelet_providers::{ModelLimitsResolver, ProviderManager, ProviderType};

/// A resolver that declares no static limit at all.
struct Undeclared;

impl ModelLimitsResolver for Undeclared {
    fn default_context_window(&self) -> Option<usize> {
        None
    }
    fn default_max_output_tokens(&self) -> Option<usize> {
        None
    }
}

#[test]
fn a_limit_with_no_user_catalog_or_declared_value_is_unavailable() {
    // @step Given a model-limits resolver that declares no default context window or output limit
    let resolver = Undeclared;

    // @step When its limits are resolved without a user value or a catalog value
    let context_window = resolve_context_window(None, None, &resolver);
    let output_limit = resolve_max_output_tokens(None, None, &resolver);

    // @step Then the context window is unavailable
    assert_eq!(context_window, None);

    // @step And the output limit is unavailable
    assert_eq!(output_limit, None);
}

#[test]
fn a_declared_provider_default_still_resolves() {
    // @step Given a Claude provider manager with no catalog or user limits
    let manager = ProviderManager::for_testing(ProviderType::Claude, None, None);

    // @step When its limits are read
    let limits = (manager.context_window(), manager.max_output_tokens());

    // @step Then the context window is 200000
    assert_eq!(limits.0, Some(200_000));

    // @step And the output limit is 8192
    assert_eq!(limits.1, Some(8_192));
}

#[test]
fn a_custom_provider_without_user_or_catalog_limits_has_no_guessed_limits() {
    // @step Given a custom provider manager with no catalog or user limits
    let manager =
        ProviderManager::for_testing(ProviderType::Custom("local-llm".to_string()), None, None);

    // @step When its limits are read
    let limits = (manager.context_window(), manager.max_output_tokens());

    // @step Then the context window is unavailable
    assert_eq!(limits.0, None);

    // @step And the output limit is unavailable
    assert_eq!(limits.1, None);
}
