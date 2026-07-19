//! PROV-101: No silent selection fallbacks — session creation paths.
//!
//! Feature: spec/features/session-creation-requires-explicit-model.feature
//!
//! These tests assert that `SessionManagerHandle::create_session` /
//! `create_isolated_session` NEVER silently default to
//! `anthropic/claude-opus-4-5` when no default model is set. With no default
//! model, creation must decline (empty SessionId) or error — no anthropic
//! substitution. With an explicit default the session adopts THAT model.
//!
//! Fully offline: a throwaway data dir is seeded with a trimmed models.json
//! fixture so `select_model` validates against the registry with no network.
//! Dummy credential env vars satisfy credential detection.

#![allow(clippy::panic)]

use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google, one tool-call
/// model each). Seeded into the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// PROV-132: Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`). Without it, a sibling test in this
/// binary that persists a `default-model.json` can swap the global pointer out
/// from under this file's `SessionManager::new()` (which eagerly loads that
/// file), leaking a foreign default model and breaking the "no default model"
/// decline assertion on the first full-suite run. Mirrors PROV-118/119/123's
/// `DATA_DIR_GUARD`; held across the synchronous critical section (the `.await`s
/// below only drive this test's own manager, not the global pointer).
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline. No
/// network calls are made — only registry validation, which reads the cache.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. The default model is intentionally NOT set; tests
/// configure it (or leave it unset) per scenario.
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
// Scenario: create_session declines when no default model is set
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_declines_when_no_default_model() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a SessionManager with no default model set
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is empty
    assert!(
        sid.value.is_empty(),
        "create_session must decline (empty id) with no default model, got '{}'",
        sid.value
    );

    // @step And no session exists in the manager
    assert!(
        manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()).is_empty(),
        "no session must be created when the default model is missing"
    );
    Ok(())
}

// =============================================================================
// Scenario: create_session uses the explicit default model, never anthropic
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_session_uses_explicit_default_model() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a SessionManager with the default model set to "google/gemini-2.5-pro"
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    manager.set_default_model("google/gemini-2.5-pro");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is not empty
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed with an explicit default model"
    );

    // @step And the created session model is "google/gemini-2.5-pro"
    let resolved = handle.get_session_model(&sid);
    assert_eq!(
        resolved.provider_id, "google",
        "provider must be the explicit google, never anthropic"
    );
    assert_eq!(
        resolved.model_id, "gemini-2.5-pro",
        "model must be the explicit gemini-2.5-pro, never claude"
    );
    Ok(())
}

// =============================================================================
// Scenario: create_isolated_session errors when no default model is set
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_isolated_session_errors_when_no_default_model() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a SessionManager with no default model set
    let (_data_dir, manager) = manager_with_seeded_cache()?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_isolated_session with no role
    let result = handle.create_isolated_session(None);

    // @step Then create_isolated_session returns an error
    assert!(
        result.is_err(),
        "create_isolated_session must error with no default model, got {result:?}"
    );

    // @step And no session exists in the manager
    assert!(
        manager.list_sessions(&std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()).is_empty(),
        "no session must be created when the default model is missing"
    );
    Ok(())
}
