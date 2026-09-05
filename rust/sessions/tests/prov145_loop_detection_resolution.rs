//! PROV-145: per-profile Loop Detection resolution in the sessions layer.
//!
//! Feature: spec/features/per-profile-loop-detection-session-wiring.feature
//! (RESOLUTION scenarios)
//!
//! RED PHASE: `codelet_sessions::model_resolution::resolve_profile_loop_detection`
//! does not exist yet, so this target fails to compile until the
//! implementation lands.
//!
//! Resolution contract:
//!   * profile model with stored values -> the flat stored values
//!     (enabled `Option<bool>`, window / maxRepeats / maxRetries `Option<u32>`)
//!   * profile model with keys absent   -> all `None` (the agent-loop layer
//!     applies the effective defaults: enabled, 160, 10, 10)
//!   * non-profile model (cloud / custom / codex) -> all `None`
//!
//! Mirrors the `prov144_max_images_resolution.rs` fixture set: a temp
//! `FSPEC_USER_DIR` with an `fspec-config.json` profile, and a
//! `ProviderManager` whose selection is the composite
//! `openai:<profile>/<model>` form.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::resolve_profile_loop_detection;
use serial_test::serial;

/// Scenario: The resolver resolves the stored loop-detection values
#[test]
#[serial]
fn profile_model_with_stored_values_resolves_them() {
    // @step Given a session against a profile storing loopDetectionEnabled false, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(
        tmp.path(),
        "work-vllm",
        Some(false),
        Some(320),
        Some(5),
        Some(2),
    );
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen", Some("work-vllm"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the per-turn loop-detection resolution runs
    let resolved = resolve_profile_loop_detection(&pm);

    // @step Then it resolves to enabled false, window 320, maxRepeats 5, maxRetries 2
    assert_eq!(resolved.enabled, Some(false));
    assert_eq!(resolved.window, Some(320));
    assert_eq!(resolved.max_repeats, Some(5));
    assert_eq!(resolved.max_retries, Some(2));
}

/// Scenario: The resolver resolves absent keys to none
#[test]
#[serial]
fn profile_model_without_keys_resolves_all_none() {
    // @step Given a session against a profile with no loop-detection keys stored
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "legacy", None, None, None, None);
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen", Some("legacy"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the per-turn loop-detection resolution runs
    let resolved = resolve_profile_loop_detection(&pm);

    // @step Then every value resolves to absent (the defaults apply downstream: enabled, 160, 10, 10)
    assert_eq!(resolved.enabled, None);
    assert_eq!(resolved.window, None);
    assert_eq!(resolved.max_repeats, None);
    assert_eq!(resolved.max_retries, None);
}

/// Scenario: A non-profile model resolves to all absent
#[test]
#[serial]
fn a_non_profile_model_resolves_to_all_absent() {
    // @step Given a session against a cloud model that has no profile behind it
    let (_tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    let pm = ProviderManager::with_provider_and_model("openai", Some("gpt-4o"), None, None)
        .expect("openai manager (sentinel key)");
    // No profile selection: a plain registry-style model string.

    // @step When the per-turn loop-detection resolution runs
    let resolved = resolve_profile_loop_detection(&pm);

    // @step Then every value resolves to absent (the RIG-014 defaults apply uniformly)
    assert_eq!(resolved.enabled, None);
    assert_eq!(resolved.window, None);
    assert_eq!(resolved.max_repeats, None);
    assert_eq!(resolved.max_retries, None);
}

/// Scenario: A mid-session switch re-resolves to the new profile's values
#[test]
#[serial]
fn a_mid_session_switch_re_resolves_to_the_new_profile() {
    // @step Given a session against a profile storing loopDetectionMaxRetries 3
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "loose", None, None, None, Some(3));
    write_profile_config(tmp.path(), "tight", None, None, None, Some(1));
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("model-a"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "model-a", Some("loose"), None, None, None)
        .expect("initial profile selection must succeed");
    assert_eq!(
        resolve_profile_loop_detection(&pm).max_retries,
        Some(3),
        "the initial profile must resolve to 3"
    );

    // @step When the session switches mid-session to a profile storing loopDetectionMaxRetries 1
    pm.set_model_direct_with_profile("openai", "model-b", Some("tight"), None, None, None)
        .expect("mid-session profile switch must succeed");

    // @step Then the resolution follows the new profile (maxRetries 1, not the previous 3)
    assert_eq!(
        resolve_profile_loop_detection(&pm).max_retries,
        Some(1),
        "after the switch the resolution must follow the new profile"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test fixtures (mirror prov144_max_images_resolution.rs)
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
fn write_profile_config(
    dir: &std::path::Path,
    profile_name: &str,
    enabled: Option<bool>,
    window: Option<u32>,
    max_repeats: Option<u32>,
    max_retries: Option<u32>,
) {
    let path = dir.join("fspec-config.json");
    let mut root: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    root["providers"]["openai"]["profiles"][profile_name] = profile_object(
        enabled, window, max_repeats, max_retries,
    );
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&root).expect("serialize"),
    )
    .expect("write fspec-config.json");
}

fn profile_object(
    enabled: Option<bool>,
    window: Option<u32>,
    max_repeats: Option<u32>,
    max_retries: Option<u32>,
) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "baseUrl": "http://192.168.0.50:8000",
        "apiKey": "sk-test",
        "contextWindow": 200000,
        "customModels": [{ "id": "qwen" }],
    });
    if let Some(v) = enabled {
        obj["loopDetectionEnabled"] = serde_json::json!(v);
    }
    if let Some(v) = window {
        obj["loopDetectionWindow"] = serde_json::json!(v);
    }
    if let Some(v) = max_repeats {
        obj["loopDetectionMaxRepeats"] = serde_json::json!(v);
    }
    if let Some(v) = max_retries {
        obj["loopDetectionMaxRetries"] = serde_json::json!(v);
    }
    obj
}
