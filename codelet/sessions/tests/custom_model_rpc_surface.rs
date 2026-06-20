//! Feature: spec/features/custom-model-rpc-surface.feature
//!
//! RPC-347 — concrete `SessionManagerHandle` custom-model write methods.
//!
//! These tests construct a real `codelet_sessions::SessionManager`, cast it to
//! `Arc<dyn codelet_core::SessionManagerHandle>`, and drive the new
//! `add_custom_model` / `update_custom_model` / `delete_custom_model` methods
//! against a temporary `fspec-config.json`. They stay offline by pointing
//! `FSPEC_USER_DIR` at a `TempDir` (serialized with `#[serial]` because the
//! env var is process-global) and never touch the network. The on-disk shape
//! assertions read the config file directly with `serde_json`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::CustomModelDefinition;
use codelet_sessions::SessionManager;
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

fn make_handle() -> Arc<dyn SessionManagerHandle> {
    Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>
}

/// Write an `fspec-config.json` into `dir` with the provided `openai` profiles
/// object and point `FSPEC_USER_DIR` at `dir`.
fn seed_config(dir: &Path, profiles: Value) {
    let root = json!({ "providers": { "openai": { "profiles": profiles } } });
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&root).unwrap(),
    )
    .unwrap();
    std::env::set_var("FSPEC_USER_DIR", dir);
}

fn read_profile(dir: &Path, name: &str) -> Value {
    let content = std::fs::read_to_string(dir.join("fspec-config.json")).unwrap();
    let root: Value = serde_json::from_str(&content).unwrap();
    root["providers"]["openai"]["profiles"][name].clone()
}

fn full_definition(id: &str) -> CustomModelDefinition {
    CustomModelDefinition {
        id: id.to_string(),
        display_name: Some(format!("{id} display")),
        facade: Some("claude".to_string()),
        context_window: Some(200_000),
        max_output_tokens: Some(8_192),
        compaction_threshold_type: Some("percentage".to_string()),
        compaction_threshold_value: Some(80),
        reasoning: Some(true),
        has_vision: Some(false),
    }
}

// Scenario: add_custom_model appends a new definition to an existing profile
#[test]
#[serial]
fn scenario_add_custom_model_appends() {
    let tmp = TempDir::new().unwrap();
    // @step Given an openai profile "work-vllm" exists with no custom models
    seed_config(
        tmp.path(),
        json!({ "work-vllm": { "baseUrl": "http://localhost:8000" } }),
    );
    let handle = make_handle();

    // @step When a client calls add_custom_model for "work-vllm" with a full CustomModelDefinition for "my-model"
    let result = handle.add_custom_model("openai", "work-vllm", &full_definition("my-model"));

    // @step Then the call returns Ok
    assert!(
        result.is_ok(),
        "add_custom_model should succeed: {result:?}"
    );

    // @step And the profile's customModels contains an entry with id "my-model" and the supplied fields
    let profile = read_profile(tmp.path(), "work-vllm");
    let models = profile["customModels"].as_array().expect("customModels[]");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["id"], "my-model");
    assert_eq!(models[0]["displayName"], "my-model display");
    assert_eq!(models[0]["facade"], "claude");
    assert_eq!(models[0]["contextWindow"], 200_000);
    assert_eq!(models[0]["compactionThreshold"]["type"], "percentage");
    assert_eq!(models[0]["compactionThreshold"]["value"], 80);
}

// Scenario: update_custom_model replaces an existing definition in place
#[test]
#[serial]
fn scenario_update_custom_model_replaces_in_place() {
    let tmp = TempDir::new().unwrap();
    // @step Given an openai profile "work-vllm" exists with custom models "alpha" then "beta"
    seed_config(
        tmp.path(),
        json!({ "work-vllm": {
            "baseUrl": "http://localhost:8000",
            "customModels": [ { "id": "alpha" }, { "id": "beta" } ]
        } }),
    );
    let handle = make_handle();

    // @step When a client calls update_custom_model for "work-vllm" with original id "alpha" and a new definition id "alpha2"
    let result =
        handle.update_custom_model("openai", "work-vllm", "alpha", &full_definition("alpha2"));

    // @step Then the call returns Ok
    assert!(
        result.is_ok(),
        "update_custom_model should succeed: {result:?}"
    );

    // @step And the customModels entry formerly "alpha" is now "alpha2" at the same array position
    let profile = read_profile(tmp.path(), "work-vllm");
    let models = profile["customModels"].as_array().expect("customModels[]");
    assert_eq!(models.len(), 2);
    assert_eq!(models[0]["id"], "alpha2");

    // @step And the entry "beta" is unchanged
    assert_eq!(models[1]["id"], "beta");
}

// Scenario: delete_custom_model removes an entry and drops the empty key
#[test]
#[serial]
fn scenario_delete_custom_model_drops_empty_key() {
    let tmp = TempDir::new().unwrap();
    // @step Given an openai profile "work-vllm" exists with a single custom model "only-model"
    seed_config(
        tmp.path(),
        json!({ "work-vllm": {
            "baseUrl": "http://localhost:8000",
            "customModels": [ { "id": "only-model" } ]
        } }),
    );
    let handle = make_handle();

    // @step When a client calls delete_custom_model for "work-vllm" with id "only-model"
    let result = handle.delete_custom_model("openai", "work-vllm", "only-model");

    // @step Then the call returns Ok
    assert!(
        result.is_ok(),
        "delete_custom_model should succeed: {result:?}"
    );

    // @step And the profile no longer has a customModels key
    let profile = read_profile(tmp.path(), "work-vllm");
    assert!(
        profile.get("customModels").is_none(),
        "customModels key should be dropped when empty, got: {profile}"
    );
}

// Scenario: delete_custom_model on a missing profile or non-openai provider is an idempotent no-op
#[test]
#[serial]
fn scenario_delete_custom_model_idempotent_no_op() {
    let tmp = TempDir::new().unwrap();
    // @step Given a config without a profile named "does-not-exist"
    seed_config(
        tmp.path(),
        json!({ "work-vllm": { "baseUrl": "http://localhost:8000" } }),
    );
    let before = std::fs::read_to_string(tmp.path().join("fspec-config.json")).unwrap();
    let handle = make_handle();

    // @step When a client calls delete_custom_model for provider "openai" profile "does-not-exist"
    let missing = handle.delete_custom_model("openai", "does-not-exist", "x");

    // @step And a client calls delete_custom_model for a non-openai provider
    let non_openai = handle.delete_custom_model("anthropic", "work-vllm", "x");

    // @step Then each call returns Ok
    assert!(
        missing.is_ok(),
        "missing-profile delete should be Ok: {missing:?}"
    );
    assert!(
        non_openai.is_ok(),
        "non-openai delete should be Ok: {non_openai:?}"
    );

    // @step And the configuration is left untouched
    let after = std::fs::read_to_string(tmp.path().join("fspec-config.json")).unwrap();
    assert_eq!(
        before, after,
        "config must be byte-identical after no-op deletes"
    );
}

// Scenario: add_custom_model and update_custom_model on a non-openai provider return an error
#[test]
#[serial]
fn scenario_add_update_non_openai_provider_errors() {
    let tmp = TempDir::new().unwrap();
    // @step Given a config with an openai profile "work-vllm"
    seed_config(
        tmp.path(),
        json!({ "work-vllm": { "baseUrl": "http://localhost:8000" } }),
    );
    let before = std::fs::read_to_string(tmp.path().join("fspec-config.json")).unwrap();
    let handle = make_handle();

    // @step When a client calls add_custom_model for a non-openai provider
    let added = handle.add_custom_model("anthropic", "work-vllm", &full_definition("m"));

    // @step And a client calls update_custom_model for a non-openai provider
    let updated = handle.update_custom_model("anthropic", "work-vllm", "m", &full_definition("m2"));

    // @step Then each call returns Err mentioning the OpenAI-only constraint
    assert!(added.is_err(), "non-openai add should be Err: {added:?}");
    assert!(
        updated.is_err(),
        "non-openai update should be Err: {updated:?}"
    );
    assert!(added.unwrap_err().contains("OpenAI"));
    assert!(updated.unwrap_err().contains("OpenAI"));

    // @step And the configuration is left untouched
    let after = std::fs::read_to_string(tmp.path().join("fspec-config.json")).unwrap();
    assert_eq!(before, after, "config must be byte-identical after errors");
}
