// Feature: spec/features/profile-auto-continue-persistence.feature
//
// PROV-142 — persistence + wire→disk bridge coverage for the per-profile
// `autoContinue` field. Exercises the read-modify-write save path against a
// real temp `fspec-config.json` and the `profile_def_from_wire` conversion.
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

fn def_auto_continue(base_url: &str, api_key: &str, auto_continue: Option<u32>) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming: None,
        auto_continue,
        preserve_thinking: None,
    }
}

/// Scenario: The wire-to-disk bridge copies the autoContinue value
#[test]
fn wire_to_disk_bridge_copies_the_auto_continue_value() {
    // @step Given a wire profile definition whose autoContinue value is 300
    let wire = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "k".to_string(),
        auto_continue: Some(300),
        ..ProfileDefinition::default()
    };

    // @step When it is converted to the on-disk profile definition
    let on_disk = profile_def_from_wire(&wire);

    // @step Then the on-disk definition carries autoContinue set to 300
    assert_eq!(on_disk.auto_continue, Some(300));
}

/// Scenario: Saving writes and removes the autoContinue key
#[test]
fn saving_writes_and_removes_the_auto_continue_key() {
    // @step Given a stored profile that has no autoContinue key
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

    // @step When the user types 0 and saves the profile (explicit off sentinel)
    save_profile_at(
        &path,
        "openai",
        "spark",
        &def_auto_continue("http://new", "new", Some(0)),
    )
    .unwrap();

    let profile = read(&path)["providers"]["openai"]["profiles"]["spark"].clone();
    // @step Then the saved config file records the autoContinue key as 0
    assert_eq!(
        profile["autoContinue"],
        Value::from(0),
        "Some(0) must be written to disk as the explicit-off sentinel"
    );

    // @step And when the profile is saved again with no autoContinue value
    save_profile_at(
        &path,
        "openai",
        "spark",
        &def_auto_continue("http://new", "new", None),
    )
    .unwrap();

    let profile = read(&path)["providers"]["openai"]["profiles"]["spark"].clone();
    // @step Then the saved config file no longer records the autoContinue key
    assert!(
        profile.get("autoContinue").is_none(),
        "None must remove the autoContinue key (absent ⇒ off, today's behavior)"
    );
}
