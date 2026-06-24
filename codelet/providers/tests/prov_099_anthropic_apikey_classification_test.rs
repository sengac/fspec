#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/anthropic-api-key-masking-derivation.feature
//!
//! PROV-099 — Management-layer half of the fix: `list_providers_info`
//! must derive `masked_key`/`source` for anthropic from its declared
//! `env_var` (`ANTHROPIC_API_KEY`) EVEN THOUGH anthropic's catalog
//! `auth_type` is `OAuth`, while OAuth-only providers that declare an
//! empty `env_var` (codex, github-copilot) stay `None`. The
//! complementary projection-layer assertions live in the
//! `codelet-fspec-tui` crate.
//!
//! All tests are `#[serial]` because they read/write process env and
//! `FSPEC_HOME`. An RAII guard saves/restores every provider env var
//! plus `FSPEC_HOME`, and points `FSPEC_HOME` at a fresh empty tempdir
//! so the real `~/.fspec` is never consulted. No network.

use std::fs;

use codelet_providers::custom::list_providers_info;
use serial_test::serial;
use tempfile::TempDir;

/// Every env var that influences provider availability/masking. Saved
/// and restored around each test so leakage cannot occur.
const PROVIDER_ENV_KEYS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    "ZAI_API_KEY",
    "ZAI_PLAN_API_KEY",
    "COHERE_API_KEY",
    "MISTRAL_API_KEY",
    "XAI_API_KEY",
    "TOGETHER_API_KEY",
    "HF_TOKEN",
    "OPENROUTER_API_KEY",
    "GROQ_API_KEY",
    "DEEPSEEK_API_KEY",
    "MOONSHOT_API_KEY",
    "GALADRIEL_API_KEY",
    "AZURE_OPENAI_API_KEY",
];

/// RAII guard: clears all provider env vars, redirects `FSPEC_HOME` to a
/// fresh empty tempdir, and restores the prior environment on drop.
struct EnvGuard {
    tempdir: TempDir,
    saved: Vec<(&'static str, Option<String>)>,
    saved_home: Option<String>,
}

impl EnvGuard {
    fn new() -> Self {
        let saved: Vec<(&'static str, Option<String>)> = PROVIDER_ENV_KEYS
            .iter()
            .map(|k| {
                let prev = std::env::var(k).ok();
                std::env::remove_var(k);
                (*k, prev)
            })
            .collect();
        let saved_home = std::env::var("FSPEC_HOME").ok();
        let tempdir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("FSPEC_HOME", tempdir.path());
        Self {
            tempdir,
            saved,
            saved_home,
        }
    }

    /// Path of the injected `FSPEC_HOME` (where `claude_auth.json` lives).
    fn home(&self) -> &std::path::Path {
        self.tempdir.path()
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        match &self.saved_home {
            Some(val) => std::env::set_var("FSPEC_HOME", val),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

// ============================================================================
// Scenario: Anthropic env API key is masked as an env-sourced credential
// ============================================================================

#[test]
#[serial]
fn anthropic_env_api_key_is_masked_as_env_sourced_credential() {
    // @step Given ANTHROPIC_API_KEY is set to "sk-ant-api03-abcdefghijklmnop" with no OAuth auth file
    let _guard = EnvGuard::new();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-api03-abcdefghijklmnop");

    // @step When list_providers_info is called
    let list = list_providers_info().expect("list_providers_info");
    let anthropic = list
        .iter()
        .find(|p| p.name == "anthropic")
        .expect("anthropic entry present in canonical registry");

    // @step Then the anthropic entry masked_key is Some("sk-ant-••••••••mnop")
    assert_eq!(
        anthropic.masked_key.as_deref(),
        Some("sk-ant-••••••••mnop"),
        "anthropic env api key must be masked despite OAuth auth_type"
    );

    // @step And the anthropic entry source is Some("env")
    assert_eq!(
        anthropic.source.as_deref(),
        Some("env"),
        "anthropic source must be 'env' when credential came from process env"
    );
}

// ============================================================================
// Scenario: Anthropic configured by an OAuth auth file carries no masked key
// ============================================================================

#[test]
#[serial]
fn anthropic_configured_by_oauth_auth_file_carries_no_masked_key() {
    // @step Given a claude_auth.json OAuth file exists under an injected FSPEC_HOME and ANTHROPIC_API_KEY is unset
    let guard = EnvGuard::new();
    let auth_json = r#"{"access_token":"oauth-access-xyz","refresh_token":"oauth-refresh-xyz","expires":9999999999999}"#;
    fs::write(guard.home().join("claude_auth.json"), auth_json).expect("write claude_auth.json");

    // @step When list_providers_info is called
    let list = list_providers_info().expect("list_providers_info");
    let anthropic = list
        .iter()
        .find(|p| p.name == "anthropic")
        .expect("anthropic entry present in canonical registry");

    // @step Then the anthropic entry available is true
    assert!(
        anthropic.available,
        "anthropic must be available via the OAuth auth file"
    );

    // @step And the anthropic entry masked_key is None
    assert!(
        anthropic.masked_key.is_none(),
        "anthropic masked_key must be None when only the OAuth file is present; got {:?}",
        anthropic.masked_key
    );

    // @step And the anthropic entry source is None
    assert!(
        anthropic.source.is_none(),
        "anthropic source must be None when only the OAuth file is present; got {:?}",
        anthropic.source
    );
}

// ============================================================================
// Scenario: OAuth-only providers with an empty env var carry no masked key
// ============================================================================

#[test]
#[serial]
fn oauth_only_providers_with_empty_env_var_carry_no_masked_key() {
    // @step Given no provider environment variables are set
    let _guard = EnvGuard::new();

    // @step When list_providers_info is called
    let list = list_providers_info().expect("list_providers_info");

    // @step Then the codex entry masked_key is None and source is None
    let codex = list
        .iter()
        .find(|p| p.name == "codex")
        .expect("codex entry present in canonical registry");
    assert!(
        codex.masked_key.is_none() && codex.source.is_none(),
        "codex must carry None masked_key/source (empty env_var); got {:?}/{:?}",
        codex.masked_key,
        codex.source
    );

    // @step And the github-copilot entry masked_key is None and source is None
    let copilot = list
        .iter()
        .find(|p| p.name == "github-copilot")
        .expect("github-copilot entry present in canonical registry");
    assert!(
        copilot.masked_key.is_none() && copilot.source.is_none(),
        "github-copilot must carry None masked_key/source (empty env_var); got {:?}/{:?}",
        copilot.masked_key,
        copilot.source
    );
}
