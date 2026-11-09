//! Offline data-dir harness shared by the RPC-002 persistence suites
//! (rpc002_session_persistence.rs and rpc002_session_resume_isolation.rs).
//! REFAC-061 moved these helpers here unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::reset_stores_for_tests;
use tempfile::TempDir;

/// Serialize tests that swap the process-global data directory.
pub static DATA_DIR_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    // BUG-186: seed the offline models.dev cache so
    // resolve_provider_manager → ModelRegistry::new → ModelCache::get()
    // stays fully offline in the fresh temp data dir (the cmpct041 /
    // rpc386 fixture pattern) — without it the registry fetch fails, the
    // create_session Err is swallowed into an empty SessionId, and the
    // Uuid::parse_str("") below panics. Dummy creds keep
    // ProviderCredentials::detect() passing offline too.
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    let cache_dir = tmp.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(
        cache_dir.join("models.json"),
        include_str!("../fixtures/prov101_models.json"),
    )
    .expect("write models fixture");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

// BUG-186: the fixture cache (fixtures/prov101_models.json) knows
// claude-opus-4-5 under anthropic — the offline-acceptable default model
// for the create_session(None) path (claude-sonnet-4 is not in the
// fixture, so registry-backed resolution would still decline offline).
pub const OFFLINE_DEFAULT_MODEL: &str = "anthropic/claude-opus-4-5";
