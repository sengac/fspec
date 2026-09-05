//! PROV-144: per-profile Max Images resolution in the sessions layer.
//!
//! Feature: spec/features/per-profile-max-images-resolution.feature
//!
//! RED PHASE: `codelet_sessions::model_resolution::resolve_profile_max_images`
//! does not exist yet, so this target fails to compile until the
//! implementation lands.
//!
//! Resolution contract:
//!   * profile model with an explicit `maxImages: n` -> `Some(n)`
//!     (including the `Some(0)` no-vision sentinel)
//!   * profile model with the key absent            -> `None`
//!     (the tool layer applies the effective default of 4)
//!   * non-profile model (cloud / custom / codex)   -> `None`
//!
//! Mirrors the `bug168_model_vision_resolution.rs` fixture set: a temp
//! `FSPEC_USER_DIR` with an `fspec-config.json` profile, and a
//! `ProviderManager` whose selection is the composite
//! `openai:<profile>/<model>` form.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::resolve_profile_max_images;
use serial_test::serial;

/// Feature: spec/features/per-profile-max-images-resolution.feature
#[test]
#[serial]
fn profile_model_with_explicit_max_images_resolves_the_stored_value() {
    // @step Given a session against a profile with maxImages 7
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "work-vllm", Some(7));
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen", Some("work-vllm"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the session creation path resolves the profile max-images value
    let max_images = resolve_profile_max_images(&pm);

    // @step Then it resolves to the stored value 7
    assert_eq!(
        max_images,
        Some(7),
        "an explicit maxImages=7 must resolve to Some(7)"
    );
}

/// Feature: spec/features/per-profile-max-images-resolution.feature
#[test]
#[serial]
fn profile_model_with_zero_max_images_resolves_the_no_vision_sentinel() {
    // @step Given a session against a profile with maxImages 0
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "no-vision", Some(0));
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen", Some("no-vision"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the session creation path resolves the profile max-images value
    let max_images = resolve_profile_max_images(&pm);

    // @step Then it resolves to the explicit 0 (no vision) rather than the default
    assert_eq!(
        max_images,
        Some(0),
        "an explicit maxImages=0 must resolve to Some(0) (the no-vision sentinel)"
    );
}

/// Feature: spec/features/per-profile-max-images-resolution.feature
#[test]
#[serial]
fn profile_model_without_max_images_key_resolves_absent() {
    // @step Given a session against a profile with no maxImages key
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "legacy", None);
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen", Some("legacy"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the session creation path resolves the profile max-images value
    let max_images = resolve_profile_max_images(&pm);

    // @step Then it resolves to absent (the tool layer applies the default 4)
    assert_eq!(
        max_images, None,
        "an absent maxImages key must resolve to None (⇒ default 4 at the tool layer)"
    );
}

/// Feature: spec/features/per-profile-max-images-resolution.feature
#[test]
#[serial]
fn a_non_profile_model_resolves_absent() {
    // @step Given a session against a cloud model that has no profile behind it
    let (_tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    let pm = ProviderManager::with_provider_and_model("openai", Some("gpt-4o"), None, None)
        .expect("openai manager (sentinel key)");
    // No profile selection: a plain registry-style model string.

    // @step When the session creation path resolves the profile max-images value
    let max_images = resolve_profile_max_images(&pm);

    // @step Then it resolves to absent (the default 4 applies uniformly)
    assert_eq!(
        max_images, None,
        "a non-profile model must resolve to None (⇒ default 4)"
    );
}

/// Feature: spec/features/per-profile-max-images-resolution.feature
#[test]
#[serial]
fn a_mid_session_switch_updates_the_resolution_to_the_new_profile() {
    // @step Given a session against a profile with maxImages 8
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "vision", Some(8));
    write_profile_config(tmp.path(), "no-vision", Some(0));
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("model-a"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "model-a", Some("vision"), None, None, None)
        .expect("initial profile selection must succeed");
    assert_eq!(
        resolve_profile_max_images(&pm),
        Some(8),
        "the initial profile must resolve to 8"
    );

    // @step When the session switches mid-session to a profile with maxImages 0
    pm.set_model_direct_with_profile("openai", "model-b", Some("no-vision"), None, None, None)
        .expect("mid-session profile switch must succeed");

    // @step Then the resolution follows the new profile (0, not the previous 8)
    assert_eq!(
        resolve_profile_max_images(&pm),
        Some(0),
        "after the switch the resolution must follow the new profile"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test fixtures (mirror bug168_model_vision_resolution.rs)
// ─────────────────────────────────────────────────────────────────────────────

struct EnvGuard {
    vars: Vec<(String, Option<std::ffi::OsString>)>,
}

impl EnvGuard {
    fn capture(keys: &[&str]) -> Self {
        let vars = keys
            .iter()
            .map(|k| (k.to_string(), std::env::var_os(k)))
            .collect();
        Self { vars }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.vars {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn tmp_dir_with_env(key: &str) -> (tempfile::TempDir, EnvGuard) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let guard = EnvGuard::capture(&[key, "OPENAI_API_KEY"]);
    std::env::set_var(key, tmp.path());
    std::env::set_var("OPENAI_API_KEY", "sk-sentinel");
    (tmp, guard)
}

/// Write (or replace) an openai profile into `<dir>/fspec-config.json`,
/// preserving any sibling profiles already present.
fn write_profile_config(dir: &std::path::Path, profile_name: &str, max_images: Option<u32>) {
    let path = dir.join("fspec-config.json");
    let mut root: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    root["providers"]["openai"]["profiles"][profile_name] = profile_object(max_images);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&root).expect("serialize"),
    )
    .expect("write fspec-config.json");
}

fn profile_object(max_images: Option<u32>) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "baseUrl": "http://192.168.0.50:8000",
        "apiKey": "sk-test",
        "contextWindow": 200000,
        "customModels": [{ "id": "qwen", "hasVision": true }],
    });
    if let Some(n) = max_images {
        obj["maxImages"] = serde_json::json!(n);
    }
    obj
}
