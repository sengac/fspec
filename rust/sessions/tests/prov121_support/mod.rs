//! Shared hermetic test setup for the PROV-121 profile-credential-bridge
//! integration tests (`prov121_profile_credential_bridge.rs`).
//!
//! Extracted into a sibling module so the single feature ↔ single test-file
//! mapping is preserved while the test file itself stays under the 300-line
//! limit (PROV-120 precedent). A sub-directory module under `tests/` is NOT
//! compiled as its own test binary, so this introduces no extra target.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_providers::ProviderManager;

pub const PROFILE_BASE_URL: &str = "http://192.168.0.50:8000";
pub const PROFILE_API_KEY: &str = "test";
pub const SENTINEL_KEY: &str = "sentinel-not-the-profile-key";

/// Save and restore the process-global env vars these tests mutate so a failure
/// (or the deliberate RED phase) can never leak state into another test.
pub struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    pub fn capture() -> Self {
        let keys = [
            "OPENAI_BASE_URL",
            "OPENAI_API_KEY",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_MODEL",
            "FSPEC_USER_DIR",
        ];
        let saved = keys
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect::<Vec<_>>();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Write an `fspec-config.json` containing one openai profile and point
/// `FSPEC_USER_DIR` at its directory so `load_local_server_profiles()` reads it.
pub fn seed_profile_config(dir: &Path) {
    let body = format!(
        r#"{{"providers":{{"openai":{{"profiles":{{"qwen":{{"baseUrl":"{PROFILE_BASE_URL}","apiKey":"{PROFILE_API_KEY}","contextWindow":200000}}}}}}}}}}"#
    );
    std::fs::write(dir.join("fspec-config.json"), body).unwrap();
    std::env::set_var("FSPEC_USER_DIR", dir);
}

/// Construct an openai `ProviderManager` without the registry.
/// `with_provider_and_model` requires openai credentials to be detectable, so a
/// sentinel `OPENAI_API_KEY` is set first — the bridge under test is expected to
/// OVERWRITE it with the profile's key.
pub fn openai_manager_with_sentinel_key() -> ProviderManager {
    std::env::set_var("OPENAI_API_KEY", SENTINEL_KEY);
    ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager should construct with a sentinel OPENAI_API_KEY present")
}
