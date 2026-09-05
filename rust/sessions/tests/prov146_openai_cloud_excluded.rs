//! PROV-146 — the OpenAI cloud catalog must never be listed under the
//! 'OpenAI API' provider section.
//!
//! Feature: spec/features/openai-cloud-catalog-excluded-from-openai-api-provider-section.feature
//!
//! Regression: a local-server openai profile's `apiKey` is bridged into the
//! `OPENAI_API_KEY` env var (PROV-121 `apply_profile_env_vars`), which made
//! `provider_has_credentials("openai")` true and caused `list_providers` to
//! populate the standalone 'OpenAI API' cloud section with the full models.dev
//! OpenAI catalog (GPT-5.6 Terra/Luna/Sol, GPT-5.5, o3, GPT-4o, ...). Those
//! cloud models belong EXCLUSIVELY under 'Codex (ChatGPT)'. The 'openai'
//! provider is for local OpenAI-protocol-compatible servers (vLLM, Ollama,
//! sglang, RunPod, Fireworks) only.
//!
//! These tests drive the public `list_providers()` surface with an isolated
//! data dir + seeded models.dev cache + isolated homes, mirroring the
//! PROV-129/PROV-130 test harness.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::cloud_models::cloud_model_entries;
use codelet_sessions::SessionManager;
use codelet_providers::models::{ModelRegistry, ModelsDevResponse};

/// models.dev catalog fixture: the openai provider carries cloud models
/// (`o3`, `gpt-4o`, `gpt-5.4`, `gpt-5.2-codex`, `gpt-5-mini`) that must
/// NEVER appear under a standalone `openai` cloud section; `gpt-5.4` +
/// `gpt-5.2-codex` are allowlisted and must appear under `codex`.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov146_openai_cloud.json");

/// Serializes tests that mutate the process-global data directory
/// (`codelet_common::set_data_directory`), the profile config env
/// (`FSPEC_USER_DIR`), and credential env vars, so a parallel test in this
/// binary cannot observe another test's seeded state.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Every credential / isolation env var these tests touch.
const API_KEY_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "CODEX_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "TOGETHER_API_KEY",
    "MOONSHOT_API_KEY",
];

fn clear_api_keys() {
    for var in API_KEY_VARS {
        std::env::remove_var(var);
    }
}

/// Seed a throwaway data dir with the offline models.dev cache and point the
/// process-global data directory at it. Returns the guard `TempDir` — keep it
/// alive for the whole test body so `build_cloud_registry` can read the cache.
fn seed_models_cache() -> tempfile::TempDir {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let cache_dir = data_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    fs::write(
        cache_dir.join("models.json"),
        MODELS_FIXTURE,
    )
    .expect("write models cache");
    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("set data directory");
    data_dir
}

/// Point `FSPEC_USER_DIR` at a temp dir holding an `fspec-config.json` with a
/// single local-server `openai` profile "sglang" carrying one custom model
/// (keeps the section reachable via MODEL-004 even though the `/v1/models`
/// probe to the bogus base URL fails).
fn seed_local_profile() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    let body = r#"{"providers":{"openai":{"profiles":{"sglang":{"baseUrl":"http://127.0.0.1:1","apiKey":"test","customModels":[{"id":"qwen3.8-27b"}]}}}}}"#;
    fs::write(dir.path().join("fspec-config.json"), body).expect("write fspec-config.json");
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    dir
}

/// Point `FSPEC_USER_DIR` at an EMPTY temp dir (no config) so no local-server
/// profile resolves.
fn seed_no_local_profile() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create empty FSPEC_USER_DIR");
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    dir
}

/// Seed a Codex OAuth `auth.json` (OAuth tokens, no cached API key) under an
/// isolated `CODEX_HOME`.
fn seed_codex_oauth() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp CODEX_HOME");
    let auth = r#"{
        "tokens": {
            "id_token": "id-tok",
            "access_token": "access-tok",
            "refresh_token": "refresh-tok",
            "account_id": "acct-123"
        }
    }"#;
    fs::write(dir.path().join("auth.json"), auth).expect("write codex auth.json");
    std::env::set_var("CODEX_HOME", dir.path());
    dir
}

/// Point `CODEX_HOME` and `FSPEC_HOME` at empty temp dirs so no Codex /
/// Copilot OAuth file of any kind resolves (the negative case).
fn seed_empty_homes() -> (tempfile::TempDir, tempfile::TempDir) {
    let codex = tempfile::tempdir().expect("create empty CODEX_HOME");
    let fspec = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("CODEX_HOME", codex.path());
    std::env::set_var("FSPEC_HOME", fspec.path());
    (codex, fspec)
}

/// Whether a standalone (non-profile) `openai` cloud section is present.
fn has_standalone_openai_section(
    providers: &[codelet_rpc_types::ProviderInfo],
) -> bool {
    providers
        .iter()
        .any(|p| p.key == "openai" && p.profile_name.is_none())
}

/// Model ids of the section with the given key, if present.
fn section_model_ids(
    providers: &[codelet_rpc_types::ProviderInfo],
    key: &str,
) -> Option<Vec<String>> {
    providers
        .iter()
        .find(|p| p.key == key)
        .map(|p| p.models.iter().map(|m| m.id.clone()).collect())
}

// =============================================================================
// Scenario: OpenAI cloud catalog is not listed under the OpenAI API provider
// when a profile bridges OPENAI_API_KEY
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_profile_bridged_key_does_not_populate_openai_cloud_section() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given a local-server openai profile named "sglang" is configured with a baseUrl and an apiKey
    let _profile_dir = seed_local_profile();
    // Isolate Codex/Copilot OAuth so no cloud OpenAI re-parenting happens.
    let (_codex_empty, _fspec_empty) = seed_empty_homes();

    // @step And the OPENAI_API_KEY environment variable is set to the profile apiKey value
    // (mirrors apply_profile_env_vars bridging the profile apiKey into env)
    std::env::set_var("OPENAI_API_KEY", "test");

    // @step And the models.dev catalog offers OpenAI models including "o3" and "gpt-4o"
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();
    let keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();

    // @step Then no section with key "openai" and no profile name appears in the list
    assert!(
        !has_standalone_openai_section(&providers),
        "PROV-146: the standalone 'openai' cloud section must not appear when a profile bridges OPENAI_API_KEY; keys: {keys:?}",
    );

    // @step And the local-server profile section "openai:sglang" is still present with its models
    let sglang = providers
        .iter()
        .find(|p| p.key == "openai:sglang")
        .unwrap_or_else(|| {
            panic!(
                "PROV-146: the local profile section 'openai:sglang' must still be present; keys: {keys:?}",
            )
        });
    assert!(
        sglang
            .models
            .iter()
            .any(|m| m.id == "qwen3.8-27b"),
        "PROV-146: the sglang profile must still list its custom model; got {:?}",
        sglang.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
    );

    // @step And no model with id "o3" appears in any non-Codex section
    for p in &providers {
        if p.key == "codex" {
            continue;
        }
        assert!(
            !p.models.iter().any(|m| m.id == "o3"),
            "PROV-146: cloud model 'o3' must not appear under the non-Codex section '{}'; keys: {keys:?}",
            p.key,
        );
    }

    clear_api_keys();
    std::env::remove_var("FSPEC_USER_DIR");
}

// =============================================================================
// Scenario: A raw OPENAI_API_KEY without profiles still yields no OpenAI API
// cloud section
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_raw_openai_key_yields_no_openai_cloud_section() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given no local-server openai profiles are configured
    let _no_profile = seed_no_local_profile();

    // @step And only an OPENAI_API_KEY is set in the environment
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");

    // @step And an ANTHROPIC_API_KEY is also set in the environment
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy");

    // @step And no Codex credentials are present
    let (_codex_empty, _fspec_empty) = seed_empty_homes();

    // @step And the models.dev catalog offers OpenAI models
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();
    let keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();

    // @step Then no section with key "openai" and no profile name appears in the list
    assert!(
        !has_standalone_openai_section(&providers),
        "PROV-146: a raw OPENAI_API_KEY (no profiles, no Codex) must still yield no 'openai' cloud section; keys: {keys:?}",
    );

    // @step And no "Codex (ChatGPT)" section is synthesized
    assert!(
        !keys.contains(&"codex"),
        "PROV-146: no Codex section may be synthesized without Codex credentials; keys: {keys:?}",
    );

    // @step And the other credentialed cloud providers still list their catalog models
    // (anthropic is seeded with a key + catalog entry, so it must survive)
    let anthropic_models = section_model_ids(&providers, "anthropic").unwrap_or_else(|| {
        panic!(
            "PROV-146: the credentialed 'anthropic' cloud section must still be populated; keys: {:?}",
            providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
        )
    });
    assert!(
        anthropic_models.iter().any(|id| id == "claude-sonnet-5"),
        "PROV-146: non-openai cloud providers must still populate from the catalog; got {anthropic_models:?}",
    );

    clear_api_keys();
    std::env::remove_var("FSPEC_USER_DIR");
}

// =============================================================================
// Scenario: Codex (ChatGPT) continues to list the allowlisted cloud OpenAI
// models when OPENAI_API_KEY is also set
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_codex_section_still_lists_allowlisted_models_when_openai_key_set() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given I am signed in with a Codex (ChatGPT) OAuth credential
    let _codex = seed_codex_oauth();
    // Isolate Copilot OAuth (FSPEC_HOME) WITHOUT disturbing the seeded CODEX_HOME.
    let fspec_empty = tempfile::tempdir().expect("empty FSPEC_HOME");
    std::env::set_var("FSPEC_HOME", fspec_empty.path());

    // @step And an OPENAI_API_KEY is set in the environment
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");

    // @step And a local-server openai profile is configured
    let _profile_dir = seed_local_profile();

    // @step And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5-mini"
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();
    let keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();
    let codex_models = section_model_ids(&providers, "codex").unwrap_or_else(|| {
        panic!("PROV-146: the 'codex' section must be present; keys: {keys:?}")
    });

    // @step Then the "Codex (ChatGPT)" section lists "gpt-5.4"
    assert!(
        codex_models.iter().any(|id| id == "gpt-5.4"),
        "PROV-146: allowlisted 'gpt-5.4' must appear under Codex; got {codex_models:?}",
    );

    // @step And the "Codex (ChatGPT)" section does not list "gpt-5-mini"
    assert!(
        !codex_models.iter().any(|id| id == "gpt-5-mini"),
        "PROV-146: non-allowlisted 'gpt-5-mini' must be filtered out of Codex; got {codex_models:?}",
    );

    // @step And no section with key "openai" and no profile name appears in the list
    assert!(
        !has_standalone_openai_section(&providers),
        "PROV-146: even with Codex creds + OPENAI_API_KEY + a profile, the standalone 'openai' cloud section must not appear; keys: {keys:?}",
    );

    // @step And the local-server profile section is still present
    assert!(
        providers.iter().any(|p| p.key == "openai:sglang"),
        "PROV-146: the local profile section 'openai:sglang' must be present; keys: {keys:?}",
    );

    clear_api_keys();
    std::env::remove_var("FSPEC_USER_DIR");
}

// =============================================================================
// Scenario: cloud_model_entries still serves the openai catalog to the Codex
// re-parenting path
// =============================================================================
#[test]
fn scenario_cloud_model_entries_still_serves_openai_catalog() {
    let _guard = ENV_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given a models.dev registry where "openai" lists a tool_call model "gpt-5.4"
    let resp: ModelsDevResponse =
        serde_json::from_str(MODELS_FIXTURE).expect("valid models.dev json");
    let registry = ModelRegistry::from_response(resp);

    // @step When cloud_model_entries is built for "openai" with credentials asserted
    let entries = cloud_model_entries(&registry, "openai", true);
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();

    // @step Then the entries contain "gpt-5.4"
    assert!(
        ids.contains(&"gpt-5.4"),
        "PROV-146: the pure helper must keep serving the openai catalog to the Codex re-parenting path; got {ids:?}",
    );

    // @step And the helper contract is unchanged (the exclusion is a list_providers call-site rule, not a helper rule)
    assert!(
        ids.contains(&"o3"),
        "PROV-146: cloud_model_entries must remain generic — the openai exclusion lives at the list_providers call site; got {ids:?}",
    );
}
