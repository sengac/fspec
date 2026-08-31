// Feature: spec/features/profile-preserve-thinking-persistence.feature
//
// PROV-143 — persistence + wire→disk bridge coverage for the per-profile
// `preserveThinking` boolean field. Exercises the read-modify-write save
// path against a real temp `fspec-config.json` and the
// `profile_def_from_wire` conversion.
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

fn def_preserve(base_url: &str, api_key: &str, preserve_thinking: Option<bool>) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking,
    }
}

/// Scenario: The wire-to-disk bridge copies the preserveThinking value
#[test]
fn wire_to_disk_bridge_copies_the_preserve_thinking_value() {
    // @step Given a wire profile definition whose preserveThinking value is true
    let wire = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "k".to_string(),
        preserve_thinking: Some(true),
        ..ProfileDefinition::default()
    };

    // @step When it is converted to the on-disk profile definition
    let on_disk = profile_def_from_wire(&wire);

    // @step Then the on-disk definition carries preserveThinking set to true
    assert_eq!(on_disk.preserve_thinking, Some(true));
}

/// Scenario: Saving writes and removes the preserveThinking key
#[test]
fn saving_writes_and_removes_the_preserve_thinking_key() {
    // @step Given a stored profile that has no preserveThinking key
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "spark": {
                    "baseUrl": "http://old",
                    "apiKey": "old"
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // @step When the user enables Preserve Thinking and saves the profile
    save_profile_at(
        &path,
        "openai",
        "spark",
        &def_preserve("http://new", "new", Some(true)),
    )
    .unwrap();

    let profile = read(&path)["providers"]["openai"]["profiles"]["spark"].clone();
    // @step Then the saved config file records the preserveThinking key as true
    assert_eq!(
        profile["preserveThinking"],
        Value::from(true),
        "Some(true) must be written to disk"
    );

    // @step And when the profile is saved again with no preserveThinking value
    save_profile_at(
        &path,
        "openai",
        "spark",
        &def_preserve("http://new", "new", None),
    )
    .unwrap();

    let profile = read(&path)["providers"]["openai"]["profiles"]["spark"].clone();
    // @step Then the saved config file has no preserveThinking key
    assert!(
        profile.get("preserveThinking").is_none(),
        "None must remove the key: {profile}"
    );
}
