//! PROV-141: Session creation without global provider credentials.
//!
//! Feature: spec/features/session-creation-without-global-credentials.feature
//!
//! Regression test for the Linux bug where `create_session` failed with
//! "No provider credentials found" whenever the default model was a
//! local-server profile model (e.g. `openai:spark/qwen3.8-27b`) and the
//! machine had no global provider credentials (no API-key env vars, no
//! OAuth auth files). Profile models carry their own `apiKey`/`baseUrl`
//! in `fspec-config.json` and bridge them into `OPENAI_*` env vars via
//! `apply_profile_env_vars` AFTER manager construction — so the global
//! `has_any()` gate in `ProviderManager::with_model_support()` was wrong.
//! It only worked on machines where a `.env` file happened to provide
//! `OPENAI_API_KEY`.
//!
//! After the fix:
//!   * Profile-model session creation succeeds with zero global credentials.
//!   * Codex-model session creation succeeds with zero global credentials.
//!   * Cloud registry models still fail loudly at selection time when the
//!     provider has no credentials (per-model validation is preserved).
//!
//! Fully offline: a throwaway data dir is seeded with a trimmed models.json
//! fixture so registry validation needs no network. All credential sources
//! (API-key env vars, OAuth auth files, custom provider configs) are
//! isolated to empty temp dirs so `ProviderCredentials::detect()` reports
//! no credentials at all.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex as AsyncMutex;

/// Trimmed offline models.dev catalog (anthropic/openai/google, one tool-call
/// model each). Seeded into the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// fspec-config.json with a single local-server profile ("spark") that stores
/// its own baseUrl and apiKey — the profile is self-sufficient and must NOT
/// require any global OPENAI_API_KEY.
const PROFILE_CONFIG: &str = r#"{
  "providers": {
    "openai": {
      "profiles": {
        "spark": {
          "baseUrl": "http://spark:8001",
          "apiKey": "test",
          "contextWindow": 262144,
          "streaming": true
        }
      }
    }
  }
}
"#;

/// Serializes the tests that swap process-global state: the data directory
/// (`codelet_common::set_data_directory`), the credential env vars, and the
/// credential-file locations (`CODEX_HOME`, `FSPEC_HOME`, `FSPEC_USER_DIR`).
/// Mirrors PROV-101's DATA_DIR_GUARD. An async mutex (not a std one) so the
/// guard is never held across an await point.
static ENV_AND_DATA_DIR_GUARD: AsyncMutex<()> = AsyncMutex::const_new(());

/// Isolate EVERY credential source so `ProviderCredentials::detect()`
/// reports zero credentials:
/// - clears all API-key env vars
/// - points CODEX_HOME / FSPEC_HOME at empty temp dirs (no OAuth auth files)
/// - points FSPEC_USER_DIR at a temp dir containing ONLY the given config
///   (no custom provider configs, no credentials dir)
///
/// Returns the temp dirs (kept alive for the test lifetime).
fn isolate_credentials(user_dir_config: &str) -> (
    tempfile::TempDir,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    for var in [
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "OPENAI_API_KEY",
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "ZAI_API_KEY",
        "ZAI_PLAN_API_KEY",
    ] {
        std::env::remove_var(var);
    }

    let codex_home = tempfile::tempdir().expect("create empty CODEX_HOME");
    std::env::set_var("CODEX_HOME", codex_home.path());

    let fspec_home = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("FSPEC_HOME", fspec_home.path());

    let user_dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    std::fs::write(user_dir.path().join("fspec-config.json"), user_dir_config)
        .expect("write fspec-config.json");
    std::env::set_var("FSPEC_USER_DIR", user_dir.path());

    (codex_home, fspec_home, user_dir)
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. Returns the temp dirs (kept alive for the test)
/// and the manager.
fn manager_with_seeded_cache(user_dir_config: &str) -> Result<(Vec<tempfile::TempDir>, Arc<SessionManager>), String> {
    let (codex_home, fspec_home, user_dir) = isolate_credentials(user_dir_config);
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    // RPC-423 precedent: reset the process-global persistence singletons
    // BEFORE swapping the data directory so the next persistence operation
    // re-initialises against the new dir (SESSION_STORE is a once-only
    // lazy singleton and would otherwise keep pointing at a prior test's
    // dropped temp dir).
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    Ok((
        vec![codex_home, fspec_home, user_dir, data_dir],
        manager,
    ))
}

// =============================================================================
// Scenario: Profile model session creation succeeds without global credentials
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn profile_model_session_creation_succeeds_without_global_credentials()
-> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with no provider credentials in the environment
    // @step And a local-server profile "spark" stored with its own baseUrl and apiKey
    // (seeded via FSPEC_USER_DIR fspec-config.json)
    let (_dirs, manager) = manager_with_seeded_cache(PROFILE_CONFIG)?;

    // @step And the default model is set to "openai:spark/qwen3.8-27b"
    manager.set_default_model("openai:spark/qwen3.8-27b");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is not empty
    assert!(
        !sid.value.is_empty(),
        "PROV-141: profile-model session creation must succeed with zero global credentials, got empty id"
    );

    // @step And the created session model is "openai:spark/qwen3.8-27b"
    // For profile models the session stores the profile-qualified provider
    // ("spark") and the bare model id ("qwen3.8-27b") — together they
    // reconstruct the composite "openai:spark/qwen3.8-27b".
    let resolved = handle.get_session_model(&sid);
    assert_eq!(
        resolved.provider_id, "spark",
        "provider must be the profile name (profile-qualified composite)"
    );
    assert_eq!(
        resolved.model_id, "qwen3.8-27b",
        "model must be the profile model qwen3.8-27b"
    );
    Ok(())
}

// =============================================================================
// Scenario: Cloud registry model still fails without credentials for its provider
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_registry_model_still_fails_without_credentials_for_its_provider()
-> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with no provider credentials in the environment
    let (_dirs, manager) = manager_with_seeded_cache("{}")?;

    // @step And the default model is set to "anthropic/claude-opus-4-5"
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is empty
    assert!(
        sid.value.is_empty(),
        "PROV-141: an uncredentialed cloud model must still fail loudly (no silent success)"
    );

    // @step And the error message names the provider "anthropic"
    // (The per-model credential check in select_model rejects the selection;
    // the observable contract is the declined session — no session created.)
    assert!(
        manager.session_count().await == 0,
        "no in-memory session must be created for an uncredentialed cloud model"
    );
    Ok(())
}

// =============================================================================
// Scenario: Codex model session creation succeeds without global credentials
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_model_session_creation_succeeds_without_global_credentials()
-> Result<(), String> {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with no provider credentials in the environment
    let (_dirs, manager) = manager_with_seeded_cache("{}")?;

    // @step And the default model is set to "codex/gpt-5"
    manager.set_default_model("codex/gpt-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is not empty
    assert!(
        !sid.value.is_empty(),
        "PROV-141: codex-model session creation must succeed with zero global credentials"
    );
    Ok(())
}
