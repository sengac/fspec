#![doc = "Feature: spec/features/provider-network-error-chain.feature"]

use codelet_cli::interactive::provider_recovery::should_retry_network_error;
use codelet_providers::codex::reconnect::{CodexNetworkFailure, NetworkFailure};

#[derive(Debug, thiserror::Error)]
#[error("Http client error: error decoding response body")]
struct BodyError {
    #[source]
    source: std::io::Error,
}

/// @step Given a provider body decoding error wraps a transport timeout
/// @step When the shared recovery policy classifies the error
/// @step Then ordinary providers can retry the transport failure
#[test]
fn wrapped_transport_timeouts_remain_retryable_for_ordinary_providers() {
    let error = anyhow::Error::new(BodyError {
        source: std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Operation timed out (os error 60)",
        ),
    })
    .context("ProviderError");
    for provider in ["openai", "anthropic", "gemini"] {
        assert!(should_retry_network_error(provider, error.as_ref()));
    }
    assert!(!should_retry_network_error("codex", error.as_ref()));
}

/// @step Given a provider body decoding error wraps invalid response data
/// @step When the shared recovery policy classifies the error
/// @step Then ordinary providers do not retry the decoding failure
#[test]
fn invalid_response_data_is_not_mistaken_for_a_transport_failure() {
    let error = anyhow::Error::new(BodyError {
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "expected value at line 1"),
    })
    .context("ProviderError");
    assert!(!should_retry_network_error("openai", error.as_ref()));
}

/// @step Given a Codex retry exhaustion error is wrapped in a transport timeout message
/// @step When the shared recovery policy classifies the error
/// @step Then no caller starts another network retry loop
#[test]
fn provider_owned_retry_exhaustion_overrides_transient_wrapper_messages() {
    let error = anyhow::Error::new(CodexNetworkFailure {
        source: NetworkFailure::Interrupted,
    })
    .context("Operation timed out (os error 60)");
    for provider in ["codex", "openai"] {
        assert!(!should_retry_network_error(provider, error.as_ref()));
    }
}
