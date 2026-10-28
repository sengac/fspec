//! Feature: spec/features/provider-config-profile-persistence.feature
//!
//! PROV-108 — concrete `SessionManagerHandle` profile write methods
//! (`save_profile` / `delete_profile`).
//!
//! Offline: a real `SessionManager` cast to `Arc<dyn SessionManagerHandle>`
//! drives the profile write surface against a temp `fspec-config.json`, with
//! `FSPEC_USER_DIR` pointed at a `TempDir` (`#[serial]` — process-global env).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;
use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;

#[path = "common/profile_rpc_helpers.rs"]
pub mod helpers;
use helpers::{
    basic_def, config_path, make_handle, point_env, read_profile, read_root, seed_config,
};

// Scenario: Saving a new profile to a missing config file creates it
#[test]
#[serial]
fn scenario_save_creates_missing_config() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given no fspec-config.json file exists
    let _env = point_env(tmp.path());
    let handle = make_handle();

    // @step When I save an openai profile "work-vllm" with baseUrl "http://localhost:8888" and apiKey "sk-test"
    let result = handle.save_profile(
        "openai",
        "work-vllm",
        &basic_def("http://localhost:8888", "sk-test"),
    );

    // @step Then the call returns Ok
    assert!(result.is_ok(), "save_profile should succeed: {result:?}");

    // @step And the profile "work-vllm" has baseUrl "http://localhost:8888"
    let profile = read_profile(tmp.path(), "work-vllm")?;
    assert_eq!(profile["baseUrl"], "http://localhost:8888");

    // @step And the profile "work-vllm" has apiKey "sk-test"
    assert_eq!(profile["apiKey"], "sk-test");
    Ok(())
}

// Scenario: Saving an existing profile preserves its custom models
#[test]
#[serial]
fn scenario_save_preserves_custom_models() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given an openai profile "work-vllm" exists with a custom model "alpha"
    let _env = seed_config(
        tmp.path(),
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://old", "apiKey": "sk-old",
                "customModels": [ { "id": "alpha" } ] }
        } } } }),
    )?;
    let handle = make_handle();

    // @step When I save the profile "work-vllm" with baseUrl "http://localhost:9999" and apiKey "sk-new"
    let result = handle.save_profile(
        "openai",
        "work-vllm",
        &basic_def("http://localhost:9999", "sk-new"),
    );

    // @step Then the call returns Ok
    assert!(result.is_ok(), "save_profile should succeed: {result:?}");

    // @step And the profile "work-vllm" has baseUrl "http://localhost:9999"
    let profile = read_profile(tmp.path(), "work-vllm")?;
    assert_eq!(profile["baseUrl"], "http://localhost:9999");

    // @step And the profile "work-vllm" still has the custom model "alpha"
    let models = profile["customModels"].as_array().expect("customModels[]");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["id"], "alpha");
    Ok(())
}

// Scenario: Saving a profile preserves sibling profiles and top-level keys
#[test]
#[serial]
fn scenario_save_preserves_siblings_and_top_level() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given a config with openai profiles "work-vllm" and "home" and a top-level "theme" key
    let _env = seed_config(
        tmp.path(),
        json!({ "theme": "dark", "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" },
            "home": { "baseUrl": "http://b", "apiKey": "k2" }
        } } } }),
    )?;
    let handle = make_handle();

    // @step When I save the profile "work-vllm" with baseUrl "http://localhost:1111" and apiKey "sk-x"
    let result = handle.save_profile(
        "openai",
        "work-vllm",
        &basic_def("http://localhost:1111", "sk-x"),
    );

    // @step Then the call returns Ok
    assert!(result.is_ok(), "save_profile should succeed: {result:?}");

    // @step And the sibling profile "home" is unchanged
    let home = read_profile(tmp.path(), "home")?;
    assert_eq!(home["baseUrl"], "http://b");
    assert_eq!(home["apiKey"], "k2");

    // @step And the top-level "theme" key is unchanged
    assert_eq!(read_root(tmp.path())?["theme"], "dark");
    Ok(())
}

// Scenario: Saving a profile writes supplied optional fields
#[test]
#[serial]
fn scenario_save_writes_optional_fields() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given no fspec-config.json file exists
    let _env = point_env(tmp.path());
    let handle = make_handle();

    // @step When I save an openai profile "work-vllm" with contextWindow 32000, maxOutputTokens 4096 and compactionThreshold percentage 80
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-test".to_string(),
        context_window: Some(32_000),
        max_output_tokens: Some(4_096),
        compaction_threshold_type: Some("percentage".to_string()),
        compaction_threshold_value: Some(80),
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
        max_images: None,
        loop_detection_enabled: None,
        loop_detection_window: None,
        loop_detection_max_repeats: None,
        loop_detection_max_retries: None,
    };
    let result = handle.save_profile("openai", "work-vllm", &def);

    // @step Then the call returns Ok
    assert!(result.is_ok(), "save_profile should succeed: {result:?}");

    let profile = read_profile(tmp.path(), "work-vllm")?;
    // @step And the profile "work-vllm" has contextWindow 32000
    assert_eq!(profile["contextWindow"], 32_000);

    // @step And the profile "work-vllm" has maxOutputTokens 4096
    assert_eq!(profile["maxOutputTokens"], 4_096);

    // @step And the profile "work-vllm" has compactionThreshold type "percentage" and value 80
    assert_eq!(profile["compactionThreshold"]["type"], "percentage");
    assert_eq!(profile["compactionThreshold"]["value"], 80);
    Ok(())
}

// Scenario: Saving a profile for a non-openai provider preserves other providers
#[test]
#[serial]
fn scenario_save_non_openai_preserves_other_providers() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = codelet_test_helpers::setup_temp_directory()?;
    // @step Given a config with an openai profile "work-vllm"
    let _env = point_env(tmp.path());
    let original = json!({ "providers": { "openai": { "profiles": {
        "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" }
    } } } });
    std::fs::write(config_path(tmp.path()), serde_json::to_vec(&original)?)?;
    let handle = make_handle();

    // @step When I save a profile for provider "anthropic"
    let result = handle.save_profile("anthropic", "work-vllm", &basic_def("http://x", "k"));

    // @step Then the call returns Ok
    assert!(result.is_ok(), "non-openai save should succeed: {result:?}");

    // @step And the anthropic profile is saved and the existing openai profile is unchanged
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config_path(tmp.path()))?)?;
    assert_eq!(
        after["providers"]["openai"],
        original["providers"]["openai"]
    );
    assert_eq!(
        after["providers"]["anthropic"]["profiles"]["work-vllm"]["baseUrl"],
        "http://x"
    );
    assert_eq!(
        after["providers"]["anthropic"]["profiles"]["work-vllm"]["apiKey"],
        "k"
    );
    Ok(())
}

// Scenario: Deleting a profile removes only the named profile
#[test]
#[serial]
fn scenario_delete_removes_only_named() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given a config with openai profiles "work-vllm" and "home"
    let _env = seed_config(
        tmp.path(),
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" },
            "home": { "baseUrl": "http://b", "apiKey": "k2" }
        } } } }),
    )?;
    let handle = make_handle();

    // @step When I delete the profile "work-vllm"
    let result = handle.delete_profile("openai", "work-vllm");

    // @step Then the call returns Ok
    assert!(result.is_ok(), "delete_profile should succeed: {result:?}");

    // @step And the profile "work-vllm" is gone
    let profiles = read_root(tmp.path())?["providers"]["openai"]["profiles"].clone();
    assert!(profiles.get("work-vllm").is_none());

    // @step And the sibling profile "home" is unchanged
    assert_eq!(profiles["home"]["baseUrl"], "http://b");
    assert_eq!(profiles["home"]["apiKey"], "k2");
    Ok(())
}

// Scenario: Deleting from a missing config file is a no-op
#[test]
#[serial]
fn scenario_delete_missing_config_no_op() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given no fspec-config.json file exists
    let _env = point_env(tmp.path());
    let handle = make_handle();

    // @step When I delete the profile "work-vllm"
    let result = handle.delete_profile("openai", "work-vllm");

    // @step Then the call returns Ok
    assert!(result.is_ok(), "delete_profile should be Ok: {result:?}");

    // @step And no fspec-config.json file is written
    assert!(!config_path(tmp.path()).exists());
    Ok(())
}

// Scenario: Deleting a non-existent profile leaves config unchanged
#[test]
#[serial]
fn scenario_delete_non_existent_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new().unwrap();
    // @step Given a config with an openai profile "work-vllm"
    let _env = seed_config(
        tmp.path(),
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" }
        } } } }),
    )?;
    let before = std::fs::read_to_string(config_path(tmp.path())).unwrap();
    let handle = make_handle();

    // @step When I delete the profile "does-not-exist"
    let result = handle.delete_profile("openai", "does-not-exist");

    // @step Then the call returns Ok
    assert!(result.is_ok(), "delete_profile should be Ok: {result:?}");

    // @step And the configuration is left byte-identical
    let after = std::fs::read_to_string(config_path(tmp.path())).unwrap();
    assert_eq!(before, after);
    Ok(())
}
