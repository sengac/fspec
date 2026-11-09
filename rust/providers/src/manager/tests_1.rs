use super::*;
use crate::copilot::auth::{write_copilot_auth, CopilotAuthJson};
use crate::models::{LimitInfo, ModelInfo, ModelRegistry, ModelsDevResponse, ProviderInfo};
use crate::test_fixtures::env::FspecHome;
use std::collections::HashMap;

// =========================================================================
// PROV-039: ProviderManager::max_output_tokens() must read runtime env var
// Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature
// =========================================================================

/// Helper to create a ProviderManager for testing without real credentials
fn test_manager(provider: ProviderType) -> ProviderManager {
    ProviderManager::for_testing(provider, None, None)
}

/// PROV-171: run `f` with runtime settings read from an empty directory and an
/// OpenAI credential present, restoring both afterwards.
fn with_openai_test_env<T>(f: impl FnOnce() -> T) -> T {
    let settings = tempfile::tempdir().expect("settings dir");
    let saved: Vec<_> = ["FSPEC_USER_DIR", "OPENAI_API_KEY"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
    std::env::set_var("FSPEC_USER_DIR", settings.path());
    std::env::set_var("OPENAI_API_KEY", "test-key");
    let result = f();
    for (key, value) in saved {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    result
}

/// PROV-171: an OpenAI manager whose OPENAI_* limit overrides were captured by
/// selecting a model, as session creation does.
fn openai_manager_after_selection() -> ProviderManager {
    let mut manager = test_manager(ProviderType::OpenAI);
    with_openai_test_env(|| manager.set_model_direct("openai", "gpt-4o", None, None, None))
        .expect("select openai model");
    manager
}

/// Build a minimal in-memory `ModelRegistry` containing a single
/// `github-copilot/gpt-4o` entry with `tool_call = true` so
/// `select_model` has something to validate against without hitting the
/// live models.dev cache.
fn build_github_copilot_registry() -> ModelRegistry {
    let mut models = HashMap::new();
    models.insert(
        "gpt-4o".to_string(),
        ModelInfo {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            family: Some("gpt-4o".to_string()),
            release_date: None,
            attachment: false,
            reasoning: false,
            tool_call: true,
            temperature: true,
            interleaved: None,
            reasoning_options: None,
            modalities: None,
            cost: None,
            limit: LimitInfo {
                context: 128_000,
                output: 16_384,
            },
            status: None,
            experimental: None,
            options: HashMap::new(),
            headers: HashMap::new(),
        },
    );
    let mut providers = HashMap::new();
    providers.insert(
        "github-copilot".to_string(),
        ProviderInfo {
            id: "github-copilot".to_string(),
            name: "GitHub Copilot".to_string(),
            env: vec![],
            npm: None,
            api: Some("https://api.githubcopilot.com".to_string()),
            doc: None,
            models,
        },
    );
    ModelRegistry::from_response(ModelsDevResponse { providers })
}

/// Build a test `ProviderManager` with the github-copilot registry
/// entry pre-loaded and **no** credentials detected — simulating the
/// state where the user hasn't logged in yet.
fn test_manager_with_copilot_registry() -> ProviderManager {
    ProviderManager {
        runtime_options: crate::ProviderRuntimeOptions::default(),
        credentials: ProviderCredentials {
            claude_available: false,
            openai_available: false,
            codex_available: false,
            gemini_available: false,
            zai_available: false,
            github_copilot_available: false,
            custom_available: std::collections::HashMap::new(),
        },
        current_provider: ProviderType::Claude,
        model_registry: Some(build_github_copilot_registry()),
        selected_model: None,
        selected_registry_provider_id: None,
        selected_profile_name: None,
        registry_context_window: None,
        registry_max_output_tokens: None,
        user_context_window: None,
        user_max_output_tokens: None,
        facade_override: None,
        compaction_threshold_override: None,
        openai_env_limits: crate::openai::OpenAIEnvLimits::default(),
        profile_reasoning: ProfileReasoning::default(),
        codex_catalog: None,
        engine_context_window: None,
    }
}

// =========================================================================
// PROV-057: ProviderManager::select_model must re-detect credentials
// Feature: spec/features/github-copilot-end-to-end-integration.feature
//
// Bug: ProviderManager snapshots ProviderCredentials::detect() once at
// construction. After OAuth login writes copilot_auth.json the cache
// still says "no credentials" until process restart.
// =========================================================================

/// Scenario: Selecting a github-copilot model right after login succeeds
/// without restart.
#[tokio::test]
#[serial_test::serial]
async fn select_model_re_detects_copilot_credentials_without_restart() {
    // @step Given ProviderManager was constructed before copilot_auth.json existed
    let _guard = FspecHome::redirect().expect("redirect FSPEC_HOME");
    let mut manager = test_manager_with_copilot_registry();

    // Sanity: the cached credentials snapshot taken at construction does
    // NOT see any Copilot credential file.
    assert!(
        !manager.credentials.has_github_copilot(),
        "precondition: no Copilot credential should be detected before login"
    );

    // @step And copilot_auth.json has just been written by the OAuth login flow
    let auth =
        CopilotAuthJson::from_github_oauth_token("gho_prov_057_fresh_login".to_string(), None);
    write_copilot_auth(&auth)
        .await
        .expect("write_copilot_auth should succeed in temp HOME");

    // @step When the user calls select_model("github-copilot/gpt-4o") in the same session
    let result = manager.select_model("github-copilot/gpt-4o");

    // @step Then ProviderCredentials::detect() is re-invoked before the has_credentials check
    // @step And the selection succeeds without a "requires credentials" error
    assert!(
        result.is_ok(),
        "select_model must succeed after a fresh copilot_auth.json is written, \
             but it returned: {:?}",
        result.err()
    );

    // And the manager must now treat github-copilot as the current provider.
    assert_eq!(manager.current_provider_name(), "github-copilot");
    assert_eq!(
        manager.selected_model_string().as_deref(),
        Some("github-copilot/gpt-4o")
    );
}

/// Regression guard: without a credential file on disk, `select_model`
/// must still surface a `requires credentials` auth error for
/// github-copilot — otherwise the stale-cache fix would silently mask
/// real missing-credential cases.
#[tokio::test]
#[serial_test::serial]
async fn select_model_still_rejects_github_copilot_when_no_credential_exists() {
    let _guard = FspecHome::redirect().expect("redirect FSPEC_HOME");
    let mut manager = test_manager_with_copilot_registry();

    let result = manager.select_model("github-copilot/gpt-4o");

    assert!(
        result.is_err(),
        "select_model must still fail when copilot_auth.json does not exist"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("requires credentials") || err.contains("Authentication"),
        "error message should mention credentials, got: {err}"
    );
}

/// PROV-171: without OPENAI_MAX_OUTPUT_TOKENS, catalog or user value the
/// OpenAI output limit is unavailable — no built-in 4096.
#[test]
#[serial_test::serial]
fn test_provider_manager_openai_max_output_tokens_unavailable_without_env_var() {
    std::env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    std::env::remove_var("OPENAI_CONTEXT_WINDOW");

    let manager = openai_manager_after_selection();

    assert_eq!(manager.max_output_tokens(), None);
    assert_eq!(manager.context_window(), None);
}

// =========================================================================
// MODEL-005: Per-Model Context Window and Max Output Configuration
// Feature: spec/features/per-model-context-window-and-max-output-configuration.feature
//
// Tests for model-specific context_window and max_output_tokens resolution.
// Priority: model-specific override > models.dev per-model > env var > provider constant.
// =========================================================================
