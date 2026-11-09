#![allow(clippy::unwrap_used, clippy::expect_used)]
//! PROV-171: OPENAI_CONTEXT_WINDOW / OPENAI_MAX_OUTPUT_TOKENS are explicit overrides —
//! unset is no override, an invalid value is an error naming the variable and value.
//!
//! Feature: spec/features/openai-compatible-local-model-support-vllm-ollama.feature

#[path = "custom_test_helpers.rs"]
mod helpers;

use codelet_providers::openai::OpenAIProvider;
use codelet_providers::{LlmProvider, ProviderError, ProviderManager, ProviderType};
use helpers::EnvGuard;
use serial_test::serial;
use std::path::Path;

const LOCAL_URL: &str = "http://localhost:8000/v1";

/// Clear both limit overrides; restored when the guards drop.
fn no_overrides() -> [EnvGuard; 2] {
    [
        EnvGuard::remove("OPENAI_CONTEXT_WINDOW"),
        EnvGuard::remove("OPENAI_MAX_OUTPUT_TOKENS"),
    ]
}

fn build_provider() -> Result<OpenAIProvider, ProviderError> {
    OpenAIProvider::from_api_key_with_options("local", "local-model", Some(LOCAL_URL), None)
}

fn assert_names(error: &ProviderError, variable: &str, value: &str) {
    let message = error.to_string();
    assert!(
        message.contains(variable) && message.contains(&format!("{value:?}")),
        "error should name {variable} and {value:?}: {message}"
    );
    assert!(
        matches!(
            error,
            ProviderError::InvalidLimitOverride { variable: v, value: got, .. }
                if *v == variable && got == value
        ),
        "expected InvalidLimitOverride, got {error:?}"
    );
}

#[test]
#[serial]
fn an_openai_provider_without_limit_overrides_reports_no_limits() {
    // @step Given OPENAI_CONTEXT_WINDOW and OPENAI_MAX_OUTPUT_TOKENS are not set
    let _env = no_overrides();

    // @step When I create an OpenAI provider
    let provider = build_provider().expect("provider without overrides");

    // @step Then the provider should report no context window
    assert_eq!(provider.context_window(), None);

    // @step And the provider should report no max output tokens
    assert_eq!(provider.max_output_tokens(), None);
}

#[test]
#[serial]
fn a_non_numeric_openai_max_output_tokens_fails_provider_construction() {
    let _env = no_overrides();
    // @step Given I set OPENAI_MAX_OUTPUT_TOKENS to "not_a_number"
    let _bad = EnvGuard::set("OPENAI_MAX_OUTPUT_TOKENS", Path::new("not_a_number"));

    // @step When I create an OpenAI provider
    let result = build_provider();

    // @step Then provider creation should fail with an error naming OPENAI_MAX_OUTPUT_TOKENS and "not_a_number"
    let error = result.expect_err("an invalid override must not build a provider");
    assert_names(&error, "OPENAI_MAX_OUTPUT_TOKENS", "not_a_number");
}

#[test]
#[serial]
fn a_zero_or_empty_openai_context_window_fails_provider_construction() {
    let _env = no_overrides();
    // @step Given I set OPENAI_CONTEXT_WINDOW to "0"
    let zero = EnvGuard::set("OPENAI_CONTEXT_WINDOW", Path::new("0"));

    // @step When I create an OpenAI provider
    let result = build_provider();

    // @step Then provider creation should fail with an error naming OPENAI_CONTEXT_WINDOW and "0"
    let error = result.expect_err("a zero override must not build a provider");
    assert_names(&error, "OPENAI_CONTEXT_WINDOW", "0");
    drop(zero);

    // @step And with OPENAI_CONTEXT_WINDOW set to "" provider creation fails naming OPENAI_CONTEXT_WINDOW
    let _empty = EnvGuard::set("OPENAI_CONTEXT_WINDOW", Path::new(""));
    let error = build_provider().expect_err("an empty override must not build a provider");
    assert_names(&error, "OPENAI_CONTEXT_WINDOW", "");
}

#[test]
#[serial]
fn an_invalid_limit_override_fails_selecting_an_openai_model() {
    let _env = no_overrides();
    let settings = tempfile::tempdir().expect("settings dir");
    let _settings = EnvGuard::set("FSPEC_USER_DIR", settings.path());
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);

    // @step Given OPENAI_CONTEXT_WINDOW is set to "32k"
    let _bad = EnvGuard::set("OPENAI_CONTEXT_WINDOW", Path::new("32k"));

    // @step When an OpenAI model is selected on the provider manager
    let result = manager.set_model_direct("openai", "local-model", None, Some(8_192), None);

    // @step Then the selection should fail with an error naming OPENAI_CONTEXT_WINDOW and "32k"
    let error = result.expect_err("an invalid override must fail the selection");
    assert_names(&error, "OPENAI_CONTEXT_WINDOW", "32k");
}
