//! Feature: spec/features/cont009-completion-contract-sync-tests-fail-because-fresh-session-returns-empty-session-id.feature
//!
//! BUG-155: the fresh_session() helper seeds the models cache so session
//! creation succeeds. The CONT-009 completion-contract tests that use the same
//! helper live in completion_contract_dispatch_arming.rs (COV-075: one test file
//! per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_core::persistence::reset_stores_for_tests;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use tokio::sync::Mutex;

const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

async fn fresh_session() -> (
    tempfile::TempDir,
    Arc<SessionManager>,
    Arc<BackgroundSession>,
) {
    // @step Given the fresh_session() helper in cont009_completion_contract_sync.rs
    // @step When the helper creates a temp data directory
    let data_dir = tempfile::tempdir().expect("tempdir");
    // @step Then it must create a cache/ subdirectory inside the temp data dir
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    // @step And it must write the prov101_models.json fixture content to cache/models.json
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models.json");
    // @step And it must call reset_stores_for_tests() before setting the data directory
    // RPC-423: reset stores BEFORE setting data directory so init_session_store()
    // creates a fresh SessionStore pointing to the new temp dir.
    reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data dir");
    // Hermetic: seed a fake key so anthropic credential validation passes
    // regardless of the developer's shell (no ambient ANTHROPIC_API_KEY).
    std::env::set_var("ANTHROPIC_API_KEY", "cont009-fake-key");
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let sid = handle
        .create_session_checked(None)
        .expect("the handle creates the session");
    // @step And the subsequent create_session() call must return a valid non-empty session ID
    assert!(
        !sid.value.is_empty(),
        "create_session must return a non-empty session id once the default model is set and the cache is seeded"
    );
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist after create_session");
    (data_dir, manager, session)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_session_seeds_the_models_cache_and_creates_a_session() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let (_dir, _manager, session) = fresh_session().await;
    assert!(!session.id.is_nil());
}
