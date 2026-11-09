//! OpenAIProvider unit tests (REFAC-031: moved from openai.rs).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::LlmProvider;

// =========================================================================
// PROV-039: OpenAI max_output_tokens must read runtime env var
// Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
// =========================================================================

/// Scenario: OpenAI max_output_tokens reads runtime environment variable
#[test]
#[serial_test::serial]
fn test_openai_provider_reads_max_output_tokens_env_var() {
    // @step Given the OPENAI_MAX_OUTPUT_TOKENS environment variable is set to "16384"
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "16384");

    // @step When an OpenAIProvider is created
    let provider =
        OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None).unwrap();

    // @step Then the instance max_output_tokens is 16384
    assert_eq!(provider.max_output_tokens(), Some(16384));

    // @step And without the environment variable the returned value is unavailable
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    let provider =
        OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None).unwrap();
    assert_eq!(provider.max_output_tokens(), None);
}

/// PROV-171: an unset OPENAI_MAX_OUTPUT_TOKENS is no override, and a
/// session-resolved limit replaces it.
#[test]
#[serial_test::serial]
fn test_openai_provider_without_env_var_reports_no_max_output_tokens() {
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");

    let provider =
        OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None).unwrap();
    assert_eq!(provider.max_output_tokens(), None);

    let limit = std::num::NonZeroUsize::new(32_768).expect("32_768 is positive");
    let provider = provider.with_max_output_tokens(limit);
    assert_eq!(provider.max_output_tokens(), Some(32_768));
}
