// Feature: spec/features/provider-settings-profile-streaming.feature
//
// PROV-139 — persistence + wire→disk bridge coverage for the per-profile
// streaming flag. Exercises the read-modify-write save path against a real
// temp `fspec-config.json` and the `profile_def_from_wire` conversion.
//
// Each Gherkin step maps to a `// @step` comment immediately above the code
// that exercises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;
use codelet_sessions::conversions::profile_def_from_wire;
use codelet_sessions::profile_persistence::{save_profile_at, ProfileDef};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn read(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn def_streaming(base_url: &str, api_key: &str, streaming: Option<bool>) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming,
        auto_continue: None,
        preserve_thinking: None,
        max_images: None,
        loop_detection_enabled: None,
        loop_detection_window: None,
        loop_detection_max_repeats: None,
        loop_detection_max_retries: None,
    }
}

/// Scenario: Saving preserves customModels while writing the streaming key
#[test]
fn saving_preserves_custom_models_while_writing_streaming_key() {
    // @step Given a stored profile that has a custom model
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "work-vllm": {
                    "baseUrl": "http://old",
                    "apiKey": "old",
                    "customModels": [ { "id": "alpha" } ]
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // @step When the user disables Streaming and saves the profile
    save_profile_at(
        &path,
        "openai",
        "work-vllm",
        &def_streaming("http://new", "new", Some(false)),
    )
    .unwrap();

    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    // @step Then the saved config file records the streaming key as disabled
    assert_eq!(profile["streaming"], Value::Bool(false));
    // @step And the saved config file still lists the custom model
    assert_eq!(profile["customModels"][0]["id"], "alpha");
}

/// Scenario: Loading a profile without a streaming key defaults to enabled
#[test]
fn loading_profile_without_streaming_key_defaults_to_enabled() {
    // @step Given a config file whose profile has no streaming key
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    // A profile written by the save path with streaming absent (None) leaves
    // out the streaming key entirely.
    save_profile_at(
        &path,
        "openai",
        "work-vllm",
        &def_streaming("http://h", "k", None),
    )
    .unwrap();
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert!(
        profile.get("streaming").is_none(),
        "streaming key must be absent when the definition carries None"
    );

    // @step When the profile is loaded
    // The wire definition reconstructed from a keyless profile carries
    // streaming = None, which the canonical helper reports as enabled.
    let loaded = ProfileDefinition {
        base_url: profile["baseUrl"].as_str().unwrap().to_string(),
        api_key: profile["apiKey"].as_str().unwrap().to_string(),
        streaming: profile.get("streaming").and_then(Value::as_bool),
        ..ProfileDefinition::default()
    };

    // @step Then the loaded profile reports streaming as enabled
    assert!(
        loaded.streaming_enabled(),
        "a profile with no streaming key must report streaming enabled"
    );
}

/// Scenario: The wire-to-disk bridge copies the streaming flag
#[test]
fn wire_to_disk_bridge_copies_the_streaming_flag() {
    // @step Given a wire profile definition whose streaming flag is set to disabled
    let wire = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "k".to_string(),
        streaming: Some(false),
        ..ProfileDefinition::default()
    };

    // @step When it is converted to the on-disk profile definition
    let on_disk = profile_def_from_wire(&wire);

    // @step Then the on-disk definition carries streaming set to disabled
    assert_eq!(on_disk.streaming, Some(false));
}
