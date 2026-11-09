//! Feature: spec/features/openai-compatible-local-model-support-vllm-ollama.feature

use codelet_providers::{LlmProvider, ProviderManager, ProviderType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn openai_session(
    max_output_tokens: Option<usize>,
) -> Result<ProviderManager, Box<dyn std::error::Error>> {
    std::env::set_var("OPENAI_API_KEY", "offline-prov164");
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(262_144), None);
    manager.set_model_direct(
        "openai",
        "qwen3.8-27b",
        Some(262_144),
        max_output_tokens,
        None,
    )?;
    Ok(manager)
}

#[test]
#[serial_test::serial]
fn a_profiles_max_output_tokens_sets_the_request_output_limit() -> TestResult {
    // @step Given an OpenAI-compatible session whose profile sets Max Output Tokens to 32768
    let manager = openai_session(Some(32_768))?;

    // @step When the OpenAI provider is built for the session
    let provider = manager.get_openai(uuid::Uuid::new_v4())?;

    // @step Then the provider should report max output tokens of 32768
    assert_eq!(provider.max_output_tokens(), Some(32_768));

    // @step And the session's output reservation is 32768
    assert_eq!(manager.max_output_tokens(), Some(32_768));
    Ok(())
}

#[test]
#[serial_test::serial]
fn without_a_profile_output_limit_the_openai_provider_is_not_built() -> TestResult {
    // @step Given an OpenAI-compatible session whose profile has no Max Output Tokens and no OPENAI_MAX_OUTPUT_TOKENS override
    let manager = openai_session(None)?;

    // @step When the OpenAI provider is built for the session
    let result = manager.get_openai(uuid::Uuid::new_v4());

    // @step Then building the provider fails because max output tokens are unavailable for the model
    let Err(error) = result else {
        return Err("a provider was built without an output limit".into());
    };
    assert!(
        error
            .to_string()
            .contains("max output tokens unavailable for openai/qwen3.8-27b"),
        "unexpected error: {error}"
    );

    // @step And the session's output reservation is unavailable
    assert_eq!(manager.max_output_tokens(), None);
    Ok(())
}
