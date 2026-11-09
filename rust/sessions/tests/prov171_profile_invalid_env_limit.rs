//! Feature: spec/features/openai-compatible-local-model-support-vllm-ollama.feature
//!
//! PROV-171 — an invalid OPENAI_MAX_OUTPUT_TOKENS fails the model selection that
//! session creation and the mid-session model switch both run
//! (`codelet_sessions::model_resolution::apply_model_selection`), naming the
//! variable and the value, instead of silently using a built-in 4096.
//!
//! Hermetic: `FSPEC_USER_DIR` points at a `TempDir` holding a seeded profile; no
//! network access occurs. Env mutations are `#[serial]` and restored.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::apply_model_selection;
use serial_test::serial;
use tempfile::TempDir;

const KEYS: [&str; 6] = [
    "OPENAI_BASE_URL",
    "OPENAI_API_KEY",
    "OPENAI_CONTEXT_WINDOW",
    "OPENAI_MAX_OUTPUT_TOKENS",
    "OPENAI_STREAMING",
    "FSPEC_USER_DIR",
];

/// Saves and restores every process-global variable this test mutates.
struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn capture() -> Self {
        Self(
            KEYS.iter()
                .map(|key| (*key, std::env::var(key).ok()))
                .collect(),
        )
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
#[serial]
fn switching_a_session_to_a_profile_model_fails_while_openai_max_output_tokens_is_invalid() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");

    // @step Given a saved OpenAI-compatible profile "qwen"
    std::fs::write(
        tmp.path().join("fspec-config.json"),
        r#"{"providers":{"openai":{"profiles":{"qwen":{"baseUrl":"http://127.0.0.1:9","apiKey":"local"}}}}}"#,
    )
    .unwrap();
    std::env::set_var("FSPEC_USER_DIR", tmp.path());
    std::env::set_var("OPENAI_API_KEY", "local");
    let mut pm = ProviderManager::with_provider("openai").expect("openai manager");

    // @step And OPENAI_MAX_OUTPUT_TOKENS is set to "lots"
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "lots");

    // @step When the session model is set to "openai:qwen/qwen"
    let result = apply_model_selection(&mut pm, "openai:qwen/qwen");

    // @step Then the model selection should fail with an error naming OPENAI_MAX_OUTPUT_TOKENS and "lots"
    let error = result.expect_err("an invalid OPENAI_MAX_OUTPUT_TOKENS must fail the selection");
    assert!(
        error.contains("OPENAI_MAX_OUTPUT_TOKENS") && error.contains("\"lots\""),
        "error should name the variable and value: {error}"
    );
}
