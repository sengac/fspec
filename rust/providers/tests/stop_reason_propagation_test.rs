#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
//!
//! PROV-039: stop_reason lost in streaming — output truncation silently treated as normal completion
//!
//! Tests for ProviderManager max_output_tokens env var reading.
//!
//! BUG-517: the streaming scenarios (Anthropic and OpenAI max_tokens,
//! truncated tool calls, normal end_turn) are tested end to end in
//! rust/agent-loop/tests/bug517_stop_reason_end_to_end.rs (a loopback
//! provider stream through the CLI stream loop into a persisted background
//! session). This file keeps the OPENAI_MAX_OUTPUT_TOKENS scenario.

use codelet_providers::{ProviderError, ProviderManager, ProviderType};

// =========================================================================
// Scenario: OpenAI max_output_tokens reads runtime environment variable
// =========================================================================

/// Scenario: OpenAI max_output_tokens reads runtime environment variable
///
/// This test calls the ACTUAL ProviderManager::max_output_tokens() method
/// to verify it reads OPENAI_MAX_OUTPUT_TOKENS when the OpenAI model is
/// selected (PROV-171: captured at selection, like the catalog limits).
#[test]
#[serial_test::serial]
fn test_provider_manager_openai_max_output_tokens_env_var() -> Result<(), ProviderError> {
    // @step Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to "16384"
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "16384");

    // @step When ProviderManager::max_output_tokens() is called for the OpenAI provider
    let result = selected_openai_manager()?.max_output_tokens();

    // @step Then the returned value is 16384
    assert_eq!(
        result,
        Some(16384),
        "max_output_tokens() should read OPENAI_MAX_OUTPUT_TOKENS env var"
    );

    // @step And without the environment variable the returned value is unavailable
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    assert_eq!(
        selected_openai_manager()?.max_output_tokens(),
        None,
        "no override, catalog or user value means the output limit is unavailable"
    );
    Ok(())
}

/// PROV-171: an invalid OPENAI_MAX_OUTPUT_TOKENS fails the OpenAI selection,
/// naming the variable and the value, instead of falling back to 4096.
#[test]
#[serial_test::serial]
fn test_provider_manager_openai_max_output_tokens_invalid_env_var() {
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "not_a_number");

    let result = selected_openai_manager();
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    let error = result.expect_err("an invalid OPENAI_MAX_OUTPUT_TOKENS must fail the selection");
    assert!(
        matches!(
            &error,
            ProviderError::InvalidLimitOverride { variable: "OPENAI_MAX_OUTPUT_TOKENS", value, .. }
                if value == "not_a_number"
        ),
        "unexpected error: {error:?}"
    );
}

/// Select an OpenAI model on a test manager with runtime settings read from an
/// empty directory, so the OPENAI_* limit overrides are captured.
fn selected_openai_manager() -> Result<ProviderManager, ProviderError> {
    let settings = tempfile::tempdir().map_err(|e| ProviderError::config("test", e.to_string()))?;
    let previous = std::env::var_os("FSPEC_USER_DIR");
    std::env::set_var("FSPEC_USER_DIR", settings.path());
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
    let result = manager.set_model_direct("openai", "gpt-4o", None, None, None);
    match previous {
        Some(value) => std::env::set_var("FSPEC_USER_DIR", value),
        None => std::env::remove_var("FSPEC_USER_DIR"),
    }
    result.map(|()| manager)
}
