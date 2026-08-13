// Feature: spec/features/openai-profile-streaming-env-bridge.feature
//
// PROV-140 — `apply_profile_env_vars` (rust/sessions/src/model_resolution.rs)
// is the single source of truth that exports a selected profile's connection
// settings as OPENAI_* environment variables. It must also export
// OPENAI_STREAMING from the loaded profile's `streaming` flag, mirroring the
// existing OPENAI_BASE_URL / OPENAI_API_KEY / OPENAI_CONTEXT_WINDOW exports. An
// absent flag leaves streaming enabled (the provider default).
//
// Offline + hermetic: `FSPEC_USER_DIR` points at a `TempDir` holding a seeded
// `fspec-config.json`; no network occurs. Because these tests mutate the
// process-global env, every test is `#[serial]` and restores what it touches.
//
// RED: `LocalServerProfile` does not yet carry `streaming`, and
// `apply_profile_env_vars` does not yet set OPENAI_STREAMING, so the
// streaming-disabled scenario fails until the bridge is wired.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_sessions::model_resolution::apply_profile_env_vars;
use serial_test::serial;
use std::path::Path;
use tempfile::TempDir;

/// Save/restore the process-global env vars these tests mutate.
struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn capture() -> Self {
        let keys = [
            "OPENAI_STREAMING",
            "OPENAI_BASE_URL",
            "OPENAI_API_KEY",
            "OPENAI_CONTEXT_WINDOW",
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

/// Write an `fspec-config.json` containing one openai profile named "qwen" and
/// point `FSPEC_USER_DIR` at its directory so `load_local_server_profiles()`
/// reads it. When `streaming` is `Some`, the camelCase `streaming` key is
/// written; when `None`, the key is omitted entirely.
fn seed_profile_config(dir: &Path, streaming: Option<bool>) {
    let streaming_field = match streaming {
        Some(v) => format!(r#","streaming":{v}"#),
        None => String::new(),
    };
    let body = format!(
        r#"{{"providers":{{"openai":{{"profiles":{{"qwen":{{"baseUrl":"http://192.168.0.50:8000","apiKey":"test"{streaming_field}}}}}}}}}}}"#
    );
    std::fs::write(dir.join("fspec-config.json"), body).unwrap();
    std::env::set_var("FSPEC_USER_DIR", dir);
}

// =============================================================================
// Scenario: Selecting a streaming-disabled profile exports OPENAI_STREAMING false
// =============================================================================
#[test]
#[serial]
fn selecting_streaming_disabled_profile_exports_openai_streaming_false() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given a stored OpenAI profile whose streaming flag is disabled
    seed_profile_config(tmp.path(), Some(false));
    std::env::remove_var("OPENAI_STREAMING");

    // @step When the profile environment variables are applied for that profile
    apply_profile_env_vars("openai", "qwen", "qwen")
        .expect("applying env vars for a known profile should succeed");

    // @step Then the OPENAI_STREAMING environment variable is set to false
    assert_eq!(
        std::env::var("OPENAI_STREAMING").ok().as_deref(),
        Some("false"),
        "a streaming-disabled profile must export OPENAI_STREAMING=false so the \
         provider selects the non-streaming request path"
    );
}

// =============================================================================
// Scenario: Selecting a profile without a streaming flag leaves streaming enabled
// =============================================================================
#[test]
#[serial]
fn selecting_profile_without_streaming_flag_leaves_streaming_enabled() {
    let _env = EnvGuard::capture();
    let tmp = TempDir::new().unwrap();

    // @step Given a stored OpenAI profile with no streaming flag
    seed_profile_config(tmp.path(), None);
    std::env::remove_var("OPENAI_STREAMING");

    // @step When the profile environment variables are applied for that profile
    apply_profile_env_vars("openai", "qwen", "qwen")
        .expect("applying env vars for a known profile should succeed");

    // @step Then the OPENAI_STREAMING environment variable does not force streaming off
    assert_ne!(
        std::env::var("OPENAI_STREAMING").ok().as_deref(),
        Some("false"),
        "a profile with no streaming flag must leave streaming enabled \
         (OPENAI_STREAMING unset or \"true\"), never forced to false"
    );
}
