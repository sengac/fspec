//! Shared offline data-dir helpers for the RPC-422 / SESS-002 session
//! persistence suites (rpc422_session_persistence.rs,
//! rpc422_session_lifecycle_persistence.rs and
//! sess002_resume_session_manifest.rs). REFAC-059 moved them here unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use tokio::sync::Mutex;

/// PROV-132: Serialize tests that swap the process-global data directory.
pub static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Trimmed offline models.dev catalog (anthropic/openai/google, one tool-call
/// model each) PLUS `claude-sonnet-4` under anthropic — the model the
/// feature file's acceptance criteria pin (`create_session_with_id` with
/// model "anthropic/claude-sonnet-4"). BUG-186: without a seeded offline
/// cache, `resolve_provider_manager → ModelRegistry::new →
/// ModelCache::get()` fails in the fresh temp data dir, the
/// `create_session` Err is swallowed into an empty SessionId, and
/// `Uuid::parse_str("")` panics below. Dummy credential env vars keep
/// `ProviderCredentials::detect()` passing offline too.
const MODELS_FIXTURE: &str = include_str!("../fixtures/rpc422_models.json");

/// Helper: create a temp data directory with a seeded offline model cache
/// and dummy credentials, and return its path.
pub fn make_temp_data_dir() -> PathBuf {
    // BUG-186: dummy creds so registry-backed selection passes offline.
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("OPENAI_API_KEY", "sk-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
    let dir = tempfile::tempdir().expect("tempdir").keep();
    let cache_dir = dir.join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("write models fixture");
    dir
}

/// Helper: set the data directory and return a guard that cleans it up.
pub fn set_temp_data_dir(path: PathBuf) -> PathBuf {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(path.clone()).expect("set_data_directory");
    // Reset the persistence singletons so they re-initialize against the new data dir.
    codelet_core::persistence::reset_stores_for_tests();
    path
}
