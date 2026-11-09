//! PROV-119 — the user-selected default model persists across process restarts.
//!
//! Feature: spec/features/default-model-persists-across-restarts.feature
//!
//! Companion to MODEL-006. The default model set via `set_default_model` was
//! previously held ONLY in an in-memory `RwLock<Option<String>>`, so every
//! fresh process started with `default_model = None` and the first
//! `create_session` was declined (PROV-101). PROV-119 persists the non-empty
//! choice to `<data_dir>/default-model.json` and reloads it at
//! `SessionManager` construction. Invariants preserved: empty/whitespace is
//! never persisted (PROV-101), a missing/malformed file loads as `None`, and a
//! persistence failure is non-fatal.
//!
//! Fully offline: a throwaway data dir is seeded with the trimmed models.json
//! fixture (shared with PROV-101) so `select_model` validates against the
//! registry with no network. Dummy credential env vars satisfy detection.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Mutex;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::default_model_persistence::load_default_model_with_dir;
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google). Seeded into
/// the temp cache so registry validation is offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serializes the tests that depend on the process-global data directory so a
/// parallel test cannot swap `codelet_common::get_data_dir()` out from under a
/// `SessionManager::new()` load. Held across the (synchronous) test body — no
/// `.await` occurs while the guard is live.
static DATA_DIR_GUARD: Mutex<()> = Mutex::new(());

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Root a fresh data dir whose model cache is pre-seeded from the offline
/// fixture. No default model is persisted. Returns the live TempDir (kept
/// alive by the caller) and its path.
fn seed_data_dir() -> (tempfile::TempDir, PathBuf) {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write fixture");
    let path = data_dir.path().to_path_buf();
    (data_dir, path)
}

// =============================================================================
// Scenario: A selected default model survives a process restart
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_default_model_survives_restart() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a data directory with no persisted default model
    let (_tmp, dir) = seed_data_dir();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(dir.clone()).expect("set data dir");
    assert!(
        load_default_model_with_dir(&dir)
            .expect("the default model loads")
            .is_none(),
        "precondition: nothing persisted yet"
    );

    // @step And a session manager whose default model is set to "anthropic/claude-opus-4-5"
    let manager1 = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager1.set_default_project(workspace)),
        Ok(())
    );
    let handle1: &dyn SessionManagerHandle = manager1.as_ref();
    assert_eq!(
        handle1.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );

    // @step When a fresh session manager is constructed against the same data directory and restores the persisted default model
    drop(manager1);
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(dir).expect("re-set data dir");
    let manager2 = SessionManager::new();
    // BUG-463: the daemon restores the persisted default after construction.
    manager2
        .restore_persisted_default_model()
        .expect("the persisted default model is restored");
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager2.set_default_project(workspace)),
        Ok(())
    );
    let handle2: &dyn SessionManagerHandle = manager2.as_ref();

    // @step Then the fresh session manager reports the default model "anthropic/claude-opus-4-5"
    assert_eq!(
        manager2.get_default_model().as_deref(),
        Some("anthropic/claude-opus-4-5"),
        "the persisted default model must load into a fresh manager"
    );

    // @step And the first create_session is no longer declined
    let sid = handle2
        .create_session_checked(None)
        .expect("the handle creates the session");
    assert!(
        !sid.value.is_empty(),
        "first create_session must succeed after restart"
    );
}

// =============================================================================
// Scenario: First launch with no persisted config has no default model
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_launch_no_config_has_no_default() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a data directory with no persisted default model
    let (_tmp, dir) = seed_data_dir();

    // @step When a session manager is constructed against that data directory
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(dir).expect("set data dir");
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step Then the session manager reports no default model
    assert!(
        manager.get_default_model().is_none(),
        "a fresh data dir must yield no default model"
    );

    // @step And the first create_session is declined until a model is selected
    let result = handle.create_session_checked(None);
    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.contains("no default model")),
        "create_session must fail naming the missing default model, got {result:?}"
    );
}

// =============================================================================
// Scenario: An empty model selection is never persisted
// =============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_model_selection_is_never_persisted() {
    let _guard = DATA_DIR_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // @step Given a data directory with no persisted default model
    let (_tmp, dir) = seed_data_dir();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(dir.clone()).expect("set data dir");

    // @step And a session manager constructed against that data directory
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When the default model is set to an empty string
    // BUG-411: an empty model is refused (and so never persisted).
    assert!(handle.set_default_model("").is_err());

    // @step Then no default model file is written to disk
    assert!(
        !dir.join("default-model.json").exists(),
        "an empty default-model string must never be persisted (PROV-101)"
    );

    // @step And the session manager reports no default model
    assert!(
        manager.get_default_model().is_none(),
        "an empty string must not set an in-memory default either"
    );
}

// =============================================================================
// Scenario: A missing default model file loads as no default model (BUG-463:
// a malformed one is an error — default-model-read-failures-are-reported)
// =============================================================================
#[test]
fn missing_default_model_file_loads_none() {
    // @step Given a data directory with no default model file
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();

    // @step When the persisted default model is loaded from that data directory
    let loaded = load_default_model_with_dir(&dir).expect("a missing file is not an error");

    // @step Then the loaded default model is none
    assert!(
        loaded.is_none(),
        "a missing default-model.json must load as None"
    );
}
