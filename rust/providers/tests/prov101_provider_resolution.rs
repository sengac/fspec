//! PROV-101: No silent selection fallbacks — provider resolution.
//!
//! Feature: spec/features/provider-resolution-no-silent-default.feature
//!
//! Asserts the Claude-first `detect_default_provider` priority chain is gone.
//! With no explicit provider selection:
//!   * exactly one credentialed provider  -> Ok(that provider) — unambiguous
//!   * more than one credentialed provider -> Err (ambiguous, no silent pick)
//!   * zero credentialed providers         -> Err (auth error)
//!
//! Exercised through the public `detect_default_provider_for_test` shim, which
//! after PROV-101 retargets to the new `resolve_unambiguous_provider`. Pure
//! in-memory `ProviderCredentials` construction — no network, no env mutation.

#![allow(clippy::panic)]

use std::collections::HashMap;

use codelet_providers::{ProviderManager, ProviderType};

fn creds(claude: bool, openai: bool, gemini: bool) -> codelet_providers::ProviderCredentials {
    codelet_providers::ProviderCredentials {
        claude_available: claude,
        openai_available: openai,
        codex_available: false,
        gemini_available: gemini,
        zai_available: false,
        github_copilot_available: false,
        custom_available: HashMap::new(),
    }
}

// =============================================================================
// Scenario: provider resolution accepts a single credentialed provider
// =============================================================================
#[test]
fn resolution_accepts_single_credentialed_provider() {
    // @step Given credentials for only the openai provider
    let credentials = creds(false, true, false);

    // @step When I resolve the provider with no explicit selection
    let result = ProviderManager::detect_default_provider_for_test(&credentials);

    // @step Then resolution succeeds with the openai provider
    match result {
        Ok(resolved) => assert_eq!(
            resolved,
            ProviderType::OpenAI,
            "single openai cred must resolve to openai, not anthropic"
        ),
        Err(err) => panic!("single credentialed provider must resolve, got {err}"),
    }
}

// =============================================================================
// Scenario: provider resolution rejects an ambiguous multi-provider state
// =============================================================================
#[test]
fn resolution_rejects_ambiguous_multi_provider() {
    // @step Given credentials for both the anthropic and openai providers
    let credentials = creds(true, true, false);

    // @step When I resolve the provider with no explicit selection
    let result = ProviderManager::detect_default_provider_for_test(&credentials);

    // @step Then resolution returns an error mentioning that none was explicitly selected
    // @step And resolution does not return the claude provider
    match result {
        Ok(ProviderType::Claude) => {
            panic!("ambiguous resolution must NOT silently return Claude")
        }
        Ok(other) => panic!("ambiguous resolution must error, got Ok({other:?})"),
        Err(err) => {
            let msg = err.to_string().to_lowercase();
            assert!(
                msg.contains("explicit"),
                "error should mention that no provider was explicitly selected, got: {err}"
            );
        }
    }
}

// =============================================================================
// Scenario: provider resolution rejects when no credentials are available
// =============================================================================
#[test]
fn resolution_rejects_when_no_credentials() {
    // @step Given no provider credentials are available
    let credentials = creds(false, false, false);

    // @step When I resolve the provider with no explicit selection
    let result = ProviderManager::detect_default_provider_for_test(&credentials);

    // @step Then resolution returns an auth error
    match result {
        Err(err) => assert!(
            err.to_string()
                .contains("No provider credentials available"),
            "error should be the no-credentials auth error, got: {err}"
        ),
        Ok(other) => panic!("zero credentials must error, got Ok({other:?})"),
    }
}
