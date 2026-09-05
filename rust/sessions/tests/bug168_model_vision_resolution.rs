#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BUG-168: vision capability resolution in the sessions layer.
//!
//! Feature: spec/features/session-model-capabilities-registry.feature
//!
//! RED PHASE: `codelet_sessions::model_resolution::resolve_model_vision`
//! does not exist yet, so this target fails to compile until the
//! implementation lands.
//!
//! Resolution contract:
//!   * registry-backed cloud/codex model -> models.dev `has_capability(Vision)`
//!   * custom provider model            -> config `ModelDef.supports_vision`
//!   * profile model                    -> profile `customModels[].hasVision`
//!   * anything unresolvable            -> false (conservative)

use codelet_providers::ProviderManager;
use codelet_sessions::model_resolution::resolve_model_vision;
use serial_test::serial;

/// Feature: spec/features/session-model-capabilities-registry.feature
#[tokio::test]
#[serial]
async fn registry_entry_set_at_session_creation_for_a_vision_model() {
    // @step Given a session is created with a cloud model the registry marks as image-capable
    let (_tmp, _guard) = tmp_data_dir();
    let mut pm = provider_manager_with_seeded_cache().await;
    pm.select_model("anthropic/claude-sonnet-4")
        .expect("registry selection");

    // @step When the session creation path resolves the model capabilities
    let vision = resolve_model_vision(&pm);

    // @step Then the tool-layer registry reports the session model supports vision
    assert!(
        vision,
        "claude-sonnet-4 is image-capable in the seed cache; resolution must be true"
    );
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn registry_entry_set_at_session_creation_for_a_non_vision_custom_model() {
    // @step Given a session is created with a custom provider model whose config sets supports_vision=false
    let (_home, _guard) = seed_custom_provider_config();
    let pm =
        ProviderManager::with_provider_and_model("mytestprov", Some("plain-model"), None, None)
            .expect("custom provider manager");

    // @step When the session creation path resolves the model capabilities
    let vision = resolve_model_vision(&pm);

    // @step Then the tool-layer registry reports the session model does not support vision
    assert!(
        !vision,
        "ModelDef.supports_vision=false must resolve to false"
    );
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn registry_entry_set_at_session_creation_for_a_vision_custom_model() {
    // @step Given a session is created with a custom provider model whose config sets supports_vision=true
    let (_home, _guard) = seed_custom_provider_config();
    let pm =
        ProviderManager::with_provider_and_model("mytestprov", Some("vision-model"), None, None)
            .expect("custom provider manager");

    // @step When the session creation path resolves the model capabilities
    let vision = resolve_model_vision(&pm);

    // @step Then the tool-layer registry reports the session model supports vision
    assert!(vision, "ModelDef.supports_vision=true must resolve to true");
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn profile_model_vision_flag_flows_from_fspec_config() {
    // @step Given an openai profile custom model declares hasVision=true in fspec-config.json
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "qwen-vision", true);
    let mut pm =
        ProviderManager::with_provider_and_model("openai", Some("qwen-vision"), None, None)
            .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile(
        "openai",
        "qwen-vision",
        Some("qwen-vision"),
        None,
        None,
        None,
    )
    .expect("profile selection must succeed");

    // @step When the session is created with that profile model
    // (profile selections route through set_model_direct_with_profile; the
    // resolver reads the profile name back off the manager's composite string)
    let vision = resolve_model_vision(&pm);

    // @step Then the tool-layer registry reports the session model supports vision
    assert!(vision, "profile hasVision=true must resolve to true");
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn profile_model_without_vision_flag_resolves_false() {
    // @step Given an openai profile custom model declares hasVision=false in fspec-config.json
    let (tmp, _guard) = tmp_dir_with_env("FSPEC_USER_DIR");
    write_profile_config(tmp.path(), "qwen-plain", false);
    let mut pm = ProviderManager::with_provider_and_model("openai", Some("qwen-plain"), None, None)
        .expect("openai manager (sentinel key)");
    pm.set_model_direct_with_profile("openai", "qwen-plain", Some("qwen-plain"), None, None, None)
        .expect("profile selection must succeed");

    // @step When the session is created with that profile model
    let vision = resolve_model_vision(&pm);

    // @step Then the tool-layer registry reports the session model does not support vision
    assert!(!vision, "profile hasVision=false must resolve to false");
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn model_switch_updates_the_resolution_to_the_new_model() {
    // @step Given a session whose registry entry currently reports vision support
    let (_home, _guard) = seed_custom_provider_config();
    let mut pm =
        ProviderManager::with_provider_and_model("mytestprov", Some("vision-model"), None, None)
            .expect("custom provider manager");
    assert!(
        resolve_model_vision(&pm),
        "initial custom model must be vision-capable"
    );

    // @step When the session switches mid-session to a model that cannot see images
    pm.set_model_direct("mytestprov", "text-only", None, None, None)
        .expect("mid-session switch must succeed");

    // @step Then the registry entry is updated to report no vision support
    assert!(
        !resolve_model_vision(&pm),
        "after the switch the resolution must follow the new model"
    );
}

/// Feature: spec/features/session-model-capabilities-registry.feature
#[test]
#[serial]
fn unresolvable_models_resolve_conservatively_to_no_vision() {
    // @step Given a provider/model pair that cannot be resolved against the registry, custom config, or profiles
    let mut pm =
        ProviderManager::with_provider_and_model("openai", Some("no-such-model"), None, None)
            .expect("openai manager (sentinel key)");
    pm.set_model_direct("openai", "no-such-model", None, None, None)
        .expect("direct selection must succeed");

    // @step When the vision capability is resolved
    let vision = resolve_model_vision(&pm);

    // @step Then it resolves to false rather than guessing true
    assert!(
        !vision,
        "unresolvable models must resolve to false (conservative)"
    );
}

/// ─────────────────────────────────────────────────────────────────────────
/// Test fixtures
/// ─────────────────────────────────────────────────────────────────────────
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

/// Fresh data dir + sentinel ANTHROPIC key; seeds the models.dev cache so
/// `ProviderManager::with_model_support()` resolves offline.
fn tmp_data_dir() -> (tempfile::TempDir, EnvGuard) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let guard = EnvGuard::capture(&["FSPEC_USER_DIR", "ANTHROPIC_API_KEY", "OPENAI_API_KEY"]);
    std::env::set_var("FSPEC_USER_DIR", tmp.path());
    std::env::set_var("ANTHROPIC_API_KEY", "sk-sentinel");
    std::env::set_var("OPENAI_API_KEY", "sk-sentinel");
    codelet_common::set_data_directory(tmp.path().to_path_buf()).expect("set_data_directory");
    seed_models_dev_cache(tmp.path());
    (tmp, guard)
}

fn tmp_dir_with_env(key: &str) -> (tempfile::TempDir, EnvGuard) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let guard = EnvGuard::capture(&[key, "OPENAI_API_KEY", "ANTHROPIC_API_KEY"]);
    std::env::set_var(key, tmp.path());
    std::env::set_var("OPENAI_API_KEY", "sk-sentinel");
    (tmp, guard)
}

/// Build a registry-backed manager over the data dir seeded by `tmp_data_dir`.
async fn provider_manager_with_seeded_cache() -> ProviderManager {
    ProviderManager::with_model_support()
        .await
        .expect("registry-backed manager (seeded cache, no network)")
}

/// Seed a models.dev cache file with one anthropic model (vision-capable).
fn seed_models_dev_cache(data_dir: &std::path::Path) {
    let cache_dir = data_dir.join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    let body = serde_json::json!({
        "anthropic": {
            "id": "anthropic",
            "name": "Anthropic",
            "env": ["ANTHROPIC_API_KEY"],
            "npm": "@anthropic-ai/sdk",
            "models": {
                "claude-sonnet-4": {
                    "id": "claude-sonnet-4-20250514",
                    "name": "Claude Sonnet 4",
                    "reasoning": true,
                    "tool_call": true,
                    "attachment": true,
                    "temperature": true,
                    "modalities": {
                        "input": ["text", "image"],
                        "output": ["text"]
                    },
                    "limit": { "context": 200000, "output": 16000 }
                }
            }
        }
    })
    .to_string();
    std::fs::write(cache_dir.join("models.json"), body).expect("write models.json");
}

/// Write two custom-provider configs into a temp `FSPEC_HOME`-style layout
/// (`<home>/providers/*.json`) and point the discovery env at it.
fn seed_custom_provider_config() -> (tempfile::TempDir, EnvGuard) {
    let tmp = tempfile::tempdir().expect("temp dir");
    let guard = EnvGuard::capture(&["FSPEC_HOME"]);
    // Discovery reads `<FSPEC_HOME-parent>/providers/*.json` when the
    // FSPEC_HOME path ends in "credentials".
    std::fs::create_dir_all(tmp.path().join("providers")).expect("providers dir");
    std::env::set_var("FSPEC_HOME", tmp.path().join("credentials"));

    let body = serde_json::json!({
        "name": "mytestprov",
        "display_name": "My Test Provider",
        "base_url": "http://localhost:9999/v1",
        "facade": "openai",
        "models": {
            "plain-model": {
                "id": "plain-model",
                "supports_vision": false
            },
            "vision-model": {
                "id": "vision-model",
                "supports_vision": true
            },
            "text-only": {
                "id": "text-only",
                "supports_vision": false
            }
        }
    })
    .to_string();
    std::fs::write(tmp.path().join("providers").join("mytestprov.json"), body)
        .expect("write custom provider config");

    (tmp, guard)
}

/// Write an fspec-config.json with one openai profile whose customModels[]
/// declares hasVision.
fn write_profile_config(dir: &std::path::Path, profile_name: &str, has_vision: bool) {
    let body = serde_json::json!({
        "providers": {
            "openai": {
                "profiles": {
                    profile_name: {
                        "baseUrl": "http://192.168.0.50:8000",
                        "apiKey": "sk-test",
                        "contextWindow": 200000,
                        "customModels": [
                            {
                                "id": profile_name,
                                "hasVision": has_vision
                            }
                        ]
                    }
                }
            }
        }
    })
    .to_string();
    std::fs::write(dir.join("fspec-config.json"), body).expect("write fspec-config.json");
}
