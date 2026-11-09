//! Feature: spec/features/openai-positive-output-limit.feature
//!
//! PROV-174 — a resolved OpenAI output limit of 0 fails provider construction
//! instead of being silently ignored. Serial: these tests set OPENAI_API_KEY.

use codelet_providers::{LlmProvider, ProviderManager, ProviderType};
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Restores OPENAI_API_KEY on drop.
struct KeyGuard(Option<std::ffi::OsString>);

impl Drop for KeyGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(key) => std::env::set_var("OPENAI_API_KEY", key),
            None => std::env::remove_var("OPENAI_API_KEY"),
        }
    }
}

/// An OpenAI manager with `gpt-4o` selected and the given output limit.
fn openai_selection(max_output_tokens: usize) -> Result<(KeyGuard, ProviderManager), String> {
    let guard = KeyGuard(std::env::var_os("OPENAI_API_KEY"));
    std::env::set_var("OPENAI_API_KEY", "sk-test-prov174");
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, None, None);
    manager
        .set_model_direct(
            "openai",
            "gpt-4o",
            Some(128_000),
            Some(max_output_tokens),
            None,
        )
        .map_err(|error| error.to_string())?;
    Ok((guard, manager))
}

#[test]
#[serial]
fn a_zero_output_limit_fails_openai_provider_construction() -> TestResult {
    // @step Given an OpenAI selection for "gpt-4o" whose resolved max output tokens is 0
    let (_guard, manager) = openai_selection(0)?;
    // @step When the session's OpenAI provider is built
    let built = manager.get_openai(uuid::Uuid::nil());
    // @step Then building fails with an error naming the zero output limit
    let error = built.err().ok_or("a zero output limit must fail")?;
    assert!(
        error
            .to_string()
            .contains("max output tokens for openai/gpt-4o is 0"),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn a_positive_output_limit_is_sent_with_the_provider() -> TestResult {
    // @step Given an OpenAI selection for "gpt-4o" whose resolved max output tokens is 16384
    let (_guard, manager) = openai_selection(16_384)?;
    // @step When the session's OpenAI provider is built
    let provider = manager.get_openai(uuid::Uuid::nil())?;
    // @step Then the provider's max output tokens is 16384
    assert_eq!(provider.max_output_tokens(), Some(16_384));
    Ok(())
}
