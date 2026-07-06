// Feature: spec/features/openai-provider-streaming-flag.feature
//
// PROV-140 — the OpenAI provider must honor the per-profile streaming choice
// sourced from the OPENAI_STREAMING environment variable. `supports_streaming()`
// is the predicate the agent runner branches on to pick the streaming vs
// non-streaming request path. Today it is hardcoded `true`, so the
// streaming-disabled scenario is RED until the field is wired.
//
// Env-var mutation is process-global, so every test is `#[serial]` and restores
// the OPENAI_* vars it touches via `EnvGuard`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::{LlmProvider, OpenAIProvider};
use serial_test::serial;

/// Save/restore the process-global env vars these tests mutate so the
/// deliberate RED failures can never leak state into another test.
struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture() -> Self {
        let keys = [
            "OPENAI_STREAMING",
            "OPENAI_BASE_URL",
            "OPENAI_API_KEY",
            "OPENAI_MODEL",
        ];
        let saved = keys
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect::<Vec<_>>();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Construct an OpenAI provider against a local base URL so any non-empty API
/// key is accepted and no network egress occurs. The constructor reads
/// `OPENAI_STREAMING` from the environment (once implemented) into the
/// provider's streaming field.
fn construct_provider() -> OpenAIProvider {
    OpenAIProvider::from_api_key_with_options(
        "sk-test-key-12345",
        "gpt-4o-mini",
        Some("http://localhost:8888"),
        None,
    )
    .expect("OpenAIProvider constructs from an api key against a local base URL")
}

// =============================================================================
// Scenario: Provider reports streaming disabled when the env flag is false
// =============================================================================
#[test]
#[serial]
fn provider_reports_streaming_disabled_when_env_flag_is_false() {
    let _env = EnvGuard::capture();

    // @step Given the OPENAI_STREAMING environment variable is set to false
    std::env::set_var("OPENAI_STREAMING", "false");

    // @step When an OpenAI provider is constructed from an api key
    let provider = construct_provider();

    // @step Then supports_streaming returns false
    assert!(
        !provider.supports_streaming(),
        "with OPENAI_STREAMING=false the provider must report streaming disabled \
         so the runtime selects the non-streaming request path"
    );
}

// =============================================================================
// Scenario: Provider defaults to streaming enabled when the env flag is unset
// =============================================================================
#[test]
#[serial]
fn provider_defaults_to_streaming_enabled_when_env_flag_is_unset() {
    let _env = EnvGuard::capture();

    // @step Given the OPENAI_STREAMING environment variable is not set
    std::env::remove_var("OPENAI_STREAMING");

    // @step When an OpenAI provider is constructed from an api key
    let provider = construct_provider();

    // @step Then supports_streaming returns true
    assert!(
        provider.supports_streaming(),
        "with OPENAI_STREAMING unset the provider must default to streaming enabled"
    );
}
