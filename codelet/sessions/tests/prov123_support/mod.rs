//! Shared hermetic test setup for the PROV-123 active-session-selection
//! integration tests (`prov123_active_selection_updates_default.rs`).
//!
//! Extracted into a sibling module so the single feature ↔ single test-file
//! mapping is preserved while the test file itself stays under the 300-line
//! limit (PROV-120/121 precedent). A sub-directory module under `tests/` is NOT
//! compiled as its own test binary, so this introduces no extra target.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;

use codelet_sessions::SessionManager;
use serde_json::Value;

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
    codelet_common::set_data_directory(data_dir.path().to_path_buf())?;
    let saved_user_dir = std::env::var("FSPEC_USER_DIR").ok();
    std::env::set_var("FSPEC_USER_DIR", data_dir.path());
    let manager = Arc::new(SessionManager::new());
    Ok((data_dir, manager, saved_user_dir))
}

/// Restore the `FSPEC_USER_DIR` env var captured by `manager_with_seeded_cache`.
pub fn restore_user_dir(saved: Option<String>) {
    match saved {
        Some(v) => std::env::set_var("FSPEC_USER_DIR", v),
        None => std::env::remove_var("FSPEC_USER_DIR"),
    }
}

/// Read `default-model.json` from `dir` as a JSON value, or `None` when absent.
pub fn read_default_model_json(dir: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(dir.join("default-model.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Read `fspec-config.json` from `dir` as a JSON value, or `None` when absent.
pub fn read_config(dir: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(dir.join("fspec-config.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Seed an openai profile "qwen" into `<dir>/fspec-config.json` so the
/// profile-qualified selection in scenario 5 resolves. Written BEFORE any
/// `set_default_model` write so the key-preserving merge keeps it.
pub fn seed_qwen_profile(dir: &Path) {
    std::fs::write(
        dir.join("fspec-config.json"),
        r#"{"providers":{"openai":{"profiles":{"qwen":{"baseUrl":"http://192.168.0.50:8000","apiKey":"profile-test-key","contextWindow":200000}}}}}"#,
    )
    .unwrap();
}
