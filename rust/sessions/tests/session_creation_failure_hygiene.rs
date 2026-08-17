//! Session-creation failure hygiene: no orphaned manifests + surfaced errors.
//!
//! Feature: spec/features/session-creation-failure-hygiene.feature
//!
//! Regression test for the Linux bug where `create_session_with_id` persisted
//! the session manifest to disk BEFORE resolving the provider. When provider
//! resolution failed (e.g. a model not present in the offline registry), the
//! manifest was already on disk but the in-memory session was never created,
//! and the error was silently swallowed by `.unwrap_or_default()` in
//! `handle_impl::create_session` — returning an empty SessionId with no log.
//!
//! After the fix:
//!   * The manifest is persisted ONLY after the in-memory session is fully
//!     constructed (provider resolution + `create_background_session_inner`
//!     succeed). A failure at any earlier step leaves NO manifest on disk.
//!   * The `handle_impl::create_session` bridge logs the underlying error via
//!     `tracing::error!` instead of silently defaulting to an empty id.
//!
//! Fully offline: a throwaway data dir is seeded with a trimmed models.json
//! fixture so registry validation is offline. Dummy credential env vars
//! satisfy credential detection.

#![allow(clippy::panic)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex as AsyncMutex;

/// Trimmed offline models.dev catalog (anthropic/openai/google, one tool-call
/// model each). Seeded into the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`). Mirrors PROV-101's DATA_DIR_GUARD.
static DATA_DIR_GUARD: AsyncMutex<()> = AsyncMutex::const_new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. Returns the temp dir (kept alive for the test)
/// and the manager.
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

/// Count the session manifest files persisted under the data dir's sessions
/// directory.
fn persisted_manifest_count(data_dir: &std::path::Path) -> usize {
    let sessions_dir = data_dir.join("sessions");
    std::fs::read_dir(&sessions_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "json")
                })
                .count()
        })
        .unwrap_or(0)
}

// =============================================================================
// Scenario: A failed session creation leaves no orphaned manifest on disk
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_session_creation_leaves_no_orphaned_manifest() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a seeded offline model cache
    let (data_dir, manager) = manager_with_seeded_cache()?;

    // @step And the default model is set to a model NOT present in the registry
    // (passes parse_model_string but fails provider resolution in
    //  create_background_session_inner)
    manager.set_default_model("anthropic/does-not-exist-model");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is empty (creation declined)
    assert!(
        sid.value.is_empty(),
        "create_session must return an empty id when provider resolution fails, got '{}'",
        sid.value
    );

    // @step And no orphaned session manifest is persisted to disk
    assert_eq!(
        persisted_manifest_count(data_dir.path()),
        0,
        "a failed session creation must NOT leave an orphaned manifest on disk"
    );

    // @step And no session exists in the manager's in-memory map
    assert!(
        manager.session_count().await == 0,
        "no in-memory session must be created when provider resolution fails"
    );
    Ok(())
}

// =============================================================================
// Scenario: A successful session creation persists exactly one manifest
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn successful_session_creation_persists_manifest() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a seeded offline model cache
    let (data_dir, manager) = manager_with_seeded_cache()?;

    // @step And the default model is set to a model present in the registry
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is not empty
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed with a valid registry model"
    );

    // @step And exactly one session manifest is persisted to disk
    assert_eq!(
        persisted_manifest_count(data_dir.path()),
        1,
        "a successful session creation must persist exactly one manifest"
    );
    Ok(())
}

// =============================================================================
// Scenario: A malformed model string leaves no orphaned manifest on disk
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_model_string_leaves_no_orphaned_manifest() -> Result<(), String> {
    let _guard = DATA_DIR_GUARD.lock().await;

    // @step Given a SessionManager with a seeded offline model cache
    let (data_dir, manager) = manager_with_seeded_cache()?;

    // @step And the default model is a malformed string (no '/')
    manager.set_default_model("malformed-model-no-slash");
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When I call create_session with no role
    let sid = handle.create_session(None);

    // @step Then the returned session id value is empty
    assert!(
        sid.value.is_empty(),
        "create_session must return an empty id for a malformed model string"
    );

    // @step And no orphaned session manifest is persisted to disk
    assert_eq!(
        persisted_manifest_count(data_dir.path()),
        0,
        "a malformed model string must NOT leave an orphaned manifest on disk"
    );
    Ok(())
}
