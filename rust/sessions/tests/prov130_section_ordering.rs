//! PROV-130 — provider section ordering + default-model selection parity.
//!
//! Feature: spec/features/model-selector-section-ordering.feature
//!
//! Rust historically assembled the `list_providers()` result cloud-first, then
//! customs, then local profiles LAST (`handle_impl.rs`). TS orders the sections
//! `[...profileSections, ...customSections, ...cloudSections]`
//! (`modelInitializationService.ts:196-200`) and, within the cloud group, leads
//! with the synthesized Codex (ChatGPT) section
//! (`cloudSectionBuilder.ts:150-155`). Because the auto-default is the first
//! section-with-models, the inverted order made the Rust default diverge from
//! TS.
//!
//! These tests drive the public `list_providers()` surface (plus the pure
//! `resolve_startup_model` default picker) with an isolated data dir + seeded
//! models.dev cache + optional local profile / Codex OAuth, and assert the
//! TS-parity DISPLAY order: profiles → custom → cloud, with Codex leading the
//! cloud group.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::startup_model_resolution::resolve_startup_model;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic: claude-opus-4-5,
/// openai: o3, google: gemini-2.5-pro) shared with PROV-101.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// models.dev catalog fixture whose OpenAI provider carries allowlisted
/// (`gpt-5.4`, `gpt-5.2-codex`) + excluded models, plus an anthropic entry.
const CODEX_MODELS_FIXTURE: &str = include_str!("fixtures/prov129_codex_models.json");

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) and mutate credential env vars, so a
/// parallel test in this binary cannot observe another test's seeded state.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Every credential / isolation env var these tests touch. Cleared at the top
/// of each test so a leaked var from a sibling test cannot change the outcome.
const API_KEY_VARS: &[&str] = &[
    "OPENAI_API_KEY",
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

/// Seed a throwaway data dir with the given offline models.dev cache and point
/// the process-global data directory at it. Returns the guard `TempDir` — keep
/// it alive for the whole test body so `build_cloud_registry` can read it.
fn seed_models_cache(fixture: &str) -> tempfile::TempDir {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let cache_dir = data_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    fs::write(cache_dir.join("models.json"), fixture).expect("write models cache");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data directory");
    data_dir
}

/// Point `FSPEC_USER_DIR` at a temp dir holding an `fspec-config.json` with a
/// single local-server `openai` profile carrying one custom model. Returns the
/// guard `TempDir`. A custom model keeps the profile reachable (MODEL-004) even
/// though the `/v1/models` probe to the bogus base URL fails.
fn seed_local_profile(profile: &str, model_id: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    let body = format!(
        r#"{{"providers":{{"openai":{{"profiles":{{"{profile}":{{"baseUrl":"http://127.0.0.1:1","customModels":[{{"id":"{model_id}"}}]}}}}}}}}}}"#
    );
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

/// Point `CODEX_HOME` + `FSPEC_HOME` at empty temp dirs so NO custom providers
/// and NO Codex/Copilot OAuth files resolve. Returns both guards.
fn seed_empty_homes() -> (tempfile::TempDir, tempfile::TempDir) {
    let codex = tempfile::tempdir().expect("create empty CODEX_HOME");
    let fspec = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("CODEX_HOME", codex.path());
    std::env::set_var("FSPEC_HOME", fspec.path());
    (codex, fspec)
}

/// Seed a Codex OAuth `auth.json` (OAuth tokens, no cached API key) under an
/// isolated `CODEX_HOME`. Mirrors what the Codex device-auth flow writes.
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

// =============================================================================
// Scenario: A local-server profile section precedes cloud sections and provides
// the default model
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn local_profile_section_precedes_cloud_and_is_default() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given a local-server openai profile with a custom model is configured
    let _profile_dir = seed_local_profile("work-vllm", "Qwen3-80B");
    // Isolate custom-provider discovery + OAuth so ONLY the seeded profile +
    // env-key cloud creds shape the result.
    let (_codex_empty, _fspec_empty) = seed_empty_homes();

    // @step And a credentialed cloud provider is present in the models.dev catalog
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy");
    let _data_dir = seed_models_cache(MODELS_FIXTURE);
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();

    // @step Then the local-server profile section appears before every cloud section
    let profile_idx = providers
        .iter()
        .position(|p| p.profile_name.is_some())
        .unwrap_or_else(|| {
            panic!(
                "PROV-130: local-server profile section missing; keys: {:?}",
                providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
            )
        });
    let first_cloud_idx = providers
        .iter()
        .position(|p| p.profile_name.is_none())
        .unwrap_or_else(|| {
            panic!(
                "PROV-130: expected at least one cloud section; keys: {:?}",
                providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
            )
        });
    assert!(
        profile_idx < first_cloud_idx,
        "PROV-130: the local-server profile section (idx {profile_idx}) must precede the first cloud section (idx {first_cloud_idx}); keys: {:?}",
        providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(),
    );

    // @step And the auto-selected default model resolves to the profile section's first model
    let resolved = resolve_startup_model(&providers, None).expect("a default model must resolve");
    assert_eq!(
        resolved.model_string, "openai:work-vllm/Qwen3-80B",
        "PROV-130: the default must be the profile section's first model (TS selectDefaultModel)",
    );

    clear_api_keys();
}

// =============================================================================
// Scenario: With no profiles or custom providers the default comes from the
// first cloud section
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn no_profiles_default_comes_from_first_cloud_section() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given no local-server profiles and no custom providers are configured
    let _no_profile = seed_no_local_profile();
    let (_codex_empty, _fspec_empty) = seed_empty_homes();

    // @step And two credentialed cloud providers are present in canonical order
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy");
    let _data_dir = seed_models_cache(MODELS_FIXTURE);
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();

    // @step Then only credentialed cloud sections appear in canonical order
    assert!(
        providers.iter().all(|p| p.profile_name.is_none()),
        "PROV-130: no profile/custom sections should appear; keys: {:?}",
        providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(),
    );
    let cloud_keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();
    let openai_pos = cloud_keys
        .iter()
        .position(|k| *k == "openai")
        .expect("openai cloud section present");
    let anthropic_pos = cloud_keys
        .iter()
        .position(|k| *k == "anthropic")
        .expect("anthropic cloud section present");
    assert!(
        openai_pos < anthropic_pos,
        "PROV-130: canonical cloud order places openai before anthropic; keys: {cloud_keys:?}",
    );

    // @step And the auto-selected default model resolves to the first populated cloud section's first model
    let resolved = resolve_startup_model(&providers, None).expect("a default model must resolve");
    assert_eq!(
        resolved.model_string, "openai/o3",
        "PROV-130: with no profiles the default is the first cloud section's first model",
    );

    clear_api_keys();
}

// =============================================================================
// Scenario: The synthesized Codex section leads the cloud group
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn synthesized_codex_section_leads_the_cloud_group() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    clear_api_keys();

    // @step Given Codex OAuth credentials are present and no profiles or custom providers are configured
    let _codex = seed_codex_oauth();
    let _no_profile = seed_no_local_profile();
    // Isolate custom-provider discovery + Copilot OAuth (FSPEC_HOME) WITHOUT
    // disturbing the seeded CODEX_HOME.
    let fspec_empty = tempfile::tempdir().expect("empty FSPEC_HOME");
    std::env::set_var("FSPEC_HOME", fspec_empty.path());

    // @step And another credentialed cloud provider Anthropic is present
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy");
    let _data_dir = seed_models_cache(CODEX_MODELS_FIXTURE);
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When list_providers() assembles the provider list
    let providers = handle.list_providers();
    let keys: Vec<&str> = providers.iter().map(|p| p.key.as_str()).collect();

    // @step Then the Codex (ChatGPT) section appears before the Anthropic cloud section
    let codex_pos = keys.iter().position(|k| *k == "codex").unwrap_or_else(|| {
        panic!("PROV-130: synthesized 'codex' section must be present; keys: {keys:?}")
    });
    let anthropic_pos = keys
        .iter()
        .position(|k| *k == "anthropic")
        .unwrap_or_else(|| {
            panic!("PROV-130: 'anthropic' cloud section must be present; keys: {keys:?}")
        });
    assert!(
        codex_pos < anthropic_pos,
        "PROV-130: the synthesized Codex section must lead the cloud group (before anthropic); keys: {keys:?}",
    );

    // @step And the standalone OpenAI section is not shown
    assert!(
        !keys.contains(&"openai"),
        "PROV-130: the standalone 'openai' section must remain removed after reordering; keys: {keys:?}",
    );

    clear_api_keys();
}
