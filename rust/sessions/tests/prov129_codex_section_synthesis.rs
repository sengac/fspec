//! PROV-129 — synthesize the Codex (ChatGPT) section by re-parenting the
//! OpenAI cloud models (TS parity).
//!
//! Feature: spec/features/codex-section-synthesis.feature
//!
//! Rust performs NO Codex-section filtering: `codex` is a static, known-absent
//! header that always renders `(0 models)` while the full models.dev OpenAI
//! catalog sits under a standalone `openai` section. TS
//! (`cloudSectionBuilder.ts` `extractCodexSection` :191-237) forces
//! `openai.hasCredentials = hasCodexOAuth || hasCodexApiKey`, then REMOVES the
//! standalone openai section and re-parents the full OpenAI models.dev catalog
//! (no allowlist filtering) under a synthetic `Codex (ChatGPT)` section.
//!
//! These tests drive the public `list_providers()` surface with an isolated
//! data dir + seeded models.dev cache + Codex OAuth file (no `OPENAI_API_KEY`),
//! and assert TS parity: a populated `codex` section, the full allowlist-free
//! OpenAI catalog, and NO standalone `openai` section (fixes discrepancies
//! #1, #5, #6). Reuses the PROV-128 unified credential predicate so Codex
//! OAuth alone yields a non-empty selectable list.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;

/// models.dev catalog fixture whose OpenAI provider carries a mix of models
/// (gpt-5.4, gpt-5.2-codex, gpt-5.1-codex, gpt-5-mini, gpt-4o) — all of which
/// must appear in the synthesized Codex section (no allowlist filtering).
const MODELS_FIXTURE: &str = include_str!("fixtures/prov129_codex_models.json");

/// Serializes tests that mutate the process-global data directory
/// (`codelet_common::set_data_directory`) and credential env vars, so a
/// parallel test in this binary cannot observe another test's seeded state.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// API-key env vars cleared so only the seeded Codex OAuth file decides Codex
/// credential presence.
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

/// Seed a throwaway data dir with the offline models.dev cache and point the
/// process-global data directory at it. Returns the guard `TempDir` — keep it
/// alive for the whole test body so `build_cloud_registry` can read the cache.
fn seed_models_cache() -> tempfile::TempDir {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let cache_dir = data_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).expect("create cache dir");
    fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models cache");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data directory");
    data_dir
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

/// Point `CODEX_HOME` and `FSPEC_HOME` at empty temp dirs so no OAuth file of
/// any kind resolves (the negative case).
fn seed_empty_homes() -> (tempfile::TempDir, tempfile::TempDir) {
    let codex = tempfile::tempdir().expect("create empty CODEX_HOME");
    let fspec = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("CODEX_HOME", codex.path());
    std::env::set_var("FSPEC_HOME", fspec.path());
    (codex, fspec)
}

/// Point `FSPEC_USER_DIR` at an empty temp dir so no local-server openai
/// profile from a real user config can leak into `list_providers()` output
/// (isolates the profile-section bucket from the cloud assertions).
fn seed_empty_user_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create empty FSPEC_USER_DIR");
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    dir
}

/// Collect the model ids of the section with the given key, if present.
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
// Scenario: Codex OAuth alone yields a populated Codex (ChatGPT) section
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn codex_oauth_yields_populated_codex_section_and_no_openai_section() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given I am signed in with a Codex (ChatGPT) OAuth credential
    let _codex = seed_codex_oauth();
    // isolate FSPEC_HOME (no copilot/claude auth) without disturbing CODEX_HOME
    let fspec_empty = tempfile::tempdir().expect("empty fspec home");
    std::env::set_var("FSPEC_HOME", fspec_empty.path());

    // @step And no OPENAI_API_KEY is set in the environment
    clear_api_keys();

    // @step And the models.dev catalog offers OpenAI models including allowlisted ones
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When I open the model selector
    let providers = handle.list_providers();

    // @step Then a "Codex (ChatGPT)" section is shown with at least one selectable model
    let codex_models = section_model_ids(&providers, "codex").unwrap_or_else(|| {
        panic!(
            "PROV-129: a synthesized 'codex' section must be present; got keys: {:?}",
            providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
        )
    });
    assert!(
        !codex_models.is_empty(),
        "PROV-129: Codex OAuth alone must yield a NON-empty Codex (ChatGPT) section (fixes #6); got {codex_models:?}",
    );
    let codex_section = providers
        .iter()
        .find(|p| p.key == "codex")
        .expect("codex section present");
    assert_eq!(
        codex_section.display_name, "Codex (ChatGPT)",
        "PROV-129: the synthetic section keeps the 'Codex (ChatGPT)' label",
    );

    // @step And no standalone "OpenAI API" section is shown
    assert!(
        section_model_ids(&providers, "openai").is_none(),
        "PROV-129: the standalone 'openai' section must be removed when Codex is synthesized",
    );
}

// =============================================================================
// Scenario: All OpenAI models from models.dev appear in Codex (ChatGPT)
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn codex_section_lists_all_openai_models_from_models_dev() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given I am signed in with a Codex (ChatGPT) OAuth credential
    let _codex = seed_codex_oauth();
    let fspec_empty = tempfile::tempdir().expect("empty fspec home");
    std::env::set_var("FSPEC_HOME", fspec_empty.path());
    clear_api_keys();

    // @step And the models.dev catalog offers the OpenAI models "gpt-5.4", "gpt-5-mini" and "gpt-4o"
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When I open the model selector
    let providers = handle.list_providers();
    let codex_models = section_model_ids(&providers, "codex").expect("codex section present");

    // @step Then the "Codex (ChatGPT)" section lists "gpt-5.4"
    assert!(
        codex_models.iter().any(|id| id == "gpt-5.4"),
        "PROV-129: 'gpt-5.4' must appear in the Codex section; got {codex_models:?}",
    );

    // @step And the "Codex (ChatGPT)" section lists "gpt-5-mini"
    // (no allowlist filtering — the full OpenAI models.dev catalog is shown)
    assert!(
        codex_models.iter().any(|id| id == "gpt-5-mini"),
        "PROV-129: the full OpenAI catalog must appear (no allowlist filter); got {codex_models:?}",
    );

    // @step And the "Codex (ChatGPT)" section lists "gpt-4o"
    assert!(
        codex_models.iter().any(|id| id == "gpt-4o"),
        "PROV-129: 'gpt-4o' must appear in the Codex section (no allowlist filter); got {codex_models:?}",
    );
}

// =============================================================================
// Scenario: Codex (ChatGPT) models are ordered newest-first by release date
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn codex_section_orders_models_newest_first() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given I am signed in with a Codex (ChatGPT) OAuth credential
    let _codex = seed_codex_oauth();
    let fspec_empty = tempfile::tempdir().expect("empty fspec home");
    std::env::set_var("FSPEC_HOME", fspec_empty.path());
    clear_api_keys();

    // @step And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5.2-codex"
    // (gpt-5.4 release_date 2026-05-01, gpt-5.2-codex release_date 2026-03-01)
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When I open the model selector
    let providers = handle.list_providers();
    let codex_models = section_model_ids(&providers, "codex").expect("codex section present");

    // @step Then the "Codex (ChatGPT)" section lists "gpt-5.4" before "gpt-5.2-codex"
    // (newest-first by release date: 2026-05-01 > 2026-03-01)
    let pos_54 = codex_models
        .iter()
        .position(|id| id == "gpt-5.4")
        .expect("gpt-5.4 present");
    let pos_52 = codex_models
        .iter()
        .position(|id| id == "gpt-5.2-codex")
        .expect("gpt-5.2-codex present");
    assert!(
        pos_54 < pos_52,
        "PROV-129: newest-first ordering must put gpt-5.4 (2026-05-01) before gpt-5.2-codex (2026-03-01); got {codex_models:?}",
    );
}

// =============================================================================
// Scenario: Without Codex credentials the standalone OpenAI API section is
// suppressed (PROV-146)
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn without_codex_credentials_openai_section_is_suppressed() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // @step Given I have no Codex credentials
    let (_codex_empty, _fspec_empty) = seed_empty_homes();
    // Isolate the profile config so the test cannot observe a real user's
    // local-server profiles (leaked openai:sglang / openai:runpod.io rows).
    let _user_dir_empty = seed_empty_user_dir();

    // @step And only an OPENAI_API_KEY is set in the environment
    clear_api_keys();
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy");

    // @step And the models.dev catalog offers OpenAI models
    let _data_dir = seed_models_cache();
    let handle: Arc<dyn SessionManagerHandle> =
        Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>;

    // @step When I open the model selector
    let providers = handle.list_providers();

    // @step Then no standalone "OpenAI API" cloud section is shown
    // (PROV-146: the openai cloud section is never populated from the
    // models.dev catalog — it is dropped by the PROV-127 filter; the openai
    // provider is for local OpenAI-protocol servers only)
    assert!(
        !providers.iter().any(|p| p.key == "openai" && p.profile_name.is_none()),
        "PROV-146: the standalone 'openai' cloud section must not appear with only OPENAI_API_KEY; got keys: {:?}",
        providers.iter().map(|p| p.key.as_str()).collect::<Vec<_>>()
    );

    // @step And no "Codex (ChatGPT)" section is synthesized
    assert!(
        section_model_ids(&providers, "codex").is_none(),
        "PROV-129: no Codex section may be synthesized without Codex credentials",
    );

    // Cleanup: drop the api key so a later test in this binary starts clean.
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("FSPEC_USER_DIR");
}
