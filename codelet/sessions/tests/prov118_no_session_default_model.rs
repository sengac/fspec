//! PROV-118: Selecting a model with no active session must set (and thereby
//! persist in-process) the default model so the next create_session succeeds.
//!
//! Feature: spec/features/set-default-model-unblocks-session-creation.feature
//!
//! Scenario covered here (the sessions-layer half):
//!   "Setting the default model unblocks session creation" — calling
//!   `SessionManagerHandle::set_default_model` with the selected provider/model
//!   string makes the subsequent `create_session` succeed (the PROV-101 decline
//!   no longer fires) and the created session adopts the selected model.
//!
//! Fully offline: a throwaway data dir is seeded with a trimmed models.json
//! fixture (shared with PROV-101) so `select_model` validates against the
//! registry with no network. Dummy credential env vars satisfy detection.

#![allow(clippy::panic)]

use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google). Seeded into
/// the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) so a parallel test cannot observe a
/// `SessionManager::new()` loading another test's persisted `default-model.json`
/// under the swapped pointer. Mirrors PROV-119's `DATA_DIR_GUARD`; held across
/// the synchronous test body (no `.await` occurs while the guard is live).
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. The default model is intentionally NOT set.
fn manager_with_seeded_cache() -> Result<(tempfile::TempDir, Arc<SessionManager>), String> {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let manager = Arc::new(SessionManager::new());
    Ok((data_dir, manager))
}

// =============================================================================
// Scenario: Setting the default model unblocks session creation
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_default_model_via_handle_unblocks_create_session() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given no session exists and no default model is set
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    assert!(
        manager.list_sessions().is_empty(),
        "precondition: no session exists"
    );
    assert!(
        handle.create_session(None).value.is_empty(),
        "precondition: create_session declines (empty id) before a default model is set"
    );

    // @step When a no-session model selection sets the default model to "anthropic/claude-opus-4-5"
    handle.set_default_model("anthropic/claude-opus-4-5");

    // @step And create_session is called
    let sid = handle.create_session(None);

    // @step Then create_session returns a non-empty session id
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed once the default model has been set"
    );

    // @step And the created session uses the model "anthropic/claude-opus-4-5"
    let resolved = handle.get_session_model(&sid);
    assert_eq!(
        resolved.provider_id, "anthropic",
        "provider must be the selected anthropic"
    );
    assert_eq!(
        resolved.model_id, "claude-opus-4-5",
        "model must be the selected claude-opus-4-5"
    );
    Ok(())
}

// =============================================================================
// Scenario (rule 5): set_default_model ignores empty strings (PROV-101 policy)
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_default_model_ignores_empty_string() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a SessionManager with no default model set
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When set_default_model is called with an empty string
    handle.set_default_model("");

    // @step Then create_session still declines (no hardcoded anthropic fallback)
    let sid = handle.create_session(None);
    assert!(
        sid.value.is_empty(),
        "an empty default-model string must be ignored — PROV-101 policy preserved"
    );
    Ok(())
}
