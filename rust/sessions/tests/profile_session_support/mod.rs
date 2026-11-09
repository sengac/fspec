//! Hermetic session-manager setup shared by the PROV-123 and BUG-318
//! profile-selection integration tests: an offline models.dev cache, a
//! fresh data dir that FSPEC_USER_DIR points at, and a seeded openai
//! profile "qwen". A sub-directory module under `tests/` is not compiled as
//! its own test binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;

use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic claude-sonnet-4 /
/// claude-opus-4-8 + openai). Seeded into the temp cache so registry validation
/// is offline.
const MODELS_FIXTURE: &str = include_str!("../fixtures/prov123_models.json");

/// Set dummy creds so `ProviderCredentials::detect()` passes offline.
pub fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("OPENAI_API_KEY", "sk-openai-test-dummy-key");
}

/// Build a manager rooted in a fresh data dir whose model cache is pre-seeded
/// from the offline fixture. Points BOTH the process-global data dir and
/// `FSPEC_USER_DIR` at that dir so disk persistence + profile lookups resolve
/// there. The default model is intentionally NOT set. Returns the kept TempDir
/// (drop = cleanup), the manager, and the saved prior `FSPEC_USER_DIR`.
pub fn manager_with_seeded_cache(
) -> Result<(tempfile::TempDir, Arc<SessionManager>, Option<String>), String> {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).map_err(|e| e.to_string())?;
    // SessionStore caches `sessions_dir` at first use — reset the
    // persistence singletons BEFORE pointing the data directory at this
    // test's fresh temp dir so session-manifest writes stay hermetic.
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let saved_user_dir = std::env::var("FSPEC_USER_DIR").ok();
    std::env::set_var("FSPEC_USER_DIR", data_dir.path());
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    manager.set_default_project(codelet_test_helpers::shared_temp_subdirectory()?)?;
    Ok((data_dir, manager, saved_user_dir))
}

/// Restore the `FSPEC_USER_DIR` env var captured by `manager_with_seeded_cache`.
pub fn restore_user_dir(saved: Option<String>) {
    match saved {
        Some(v) => std::env::set_var("FSPEC_USER_DIR", v),
        None => std::env::remove_var("FSPEC_USER_DIR"),
    }
}

/// Seed an openai profile "qwen" into `<dir>/fspec-config.json` so the
/// profile-qualified selection in scenario 5 resolves. Written BEFORE any
/// `set_default_model` write so the key-preserving merge keeps it.
pub fn seed_qwen_profile(dir: &Path) {
    std::fs::write(
        dir.join("fspec-config.json"),
        r#"{"providers":{"openai":{"profiles":{"qwen":{"baseUrl":"http://192.168.0.50:8000","apiKey":"profile-test-key","contextWindow":200000,"maxOutputTokens":16384}}}}}"#,
    )
    .unwrap();
}
