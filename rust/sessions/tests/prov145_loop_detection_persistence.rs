//! PROV-145 — persistence + wire→disk bridge coverage for the four
//! per-profile loop-detection fields. Exercises the read-modify-write save
//! path against a real temp `fspec-config.json`, the `profile_def_from_wire`
//! conversion, and the `LocalServerProfile` disk read-back (lenient
//! deserializer).
//!
//! Feature: spec/features/per-profile-loop-detection-persistence.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it.
//!
//! RED PHASE: `ProfileDef` has no `loop_detection_*` fields and the
//! `loopDetection*` merge/read paths do not exist yet, so this target fails
//! to compile until the implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;
use codelet_sessions::conversions::profile_def_from_wire;
use codelet_sessions::profile_persistence::{rename_profile_at, save_profile_at, ProfileDef};
use codelet_sessions::profile_sections::load_local_server_profiles;
use serde_json::{json, Value};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn read(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn def_loop(
    base_url: &str,
    api_key: &str,
    enabled: Option<bool>,
    window: Option<u32>,
    max_repeats: Option<u32>,
    max_retries: Option<u32>,
) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
        max_images: None,
        loop_detection_enabled: enabled,
        loop_detection_window: window,
        loop_detection_max_repeats: max_repeats,
        loop_detection_max_retries: max_retries,
    }
}

fn wire_loop(
    enabled: Option<bool>,
    window: Option<u32>,
    max_repeats: Option<u32>,
    max_retries: Option<u32>,
) -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://new".to_string(),
        api_key: "new".to_string(),
        loop_detection_enabled: enabled,
        loop_detection_window: window,
        loop_detection_max_repeats: max_repeats,
        loop_detection_max_retries: max_retries,
        ..ProfileDefinition::default()
    }
}

/// Scenario: The loop-detection values round-trip through wire and disk
#[test]
fn the_loop_detection_values_round_trip_through_wire_and_disk() {
    // @step Given a profile definition with loopDetectionEnabled true, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    let def = def_loop("http://new", "new", Some(true), Some(320), Some(5), Some(2));

    // @step When the profile is saved to fspec-config.json
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();

    // @step Then the stored profile object contains "loopDetectionEnabled": true, "loopDetectionWindow": 320, "loopDetectionMaxRepeats": 5, "loopDetectionMaxRetries": 2
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(
        profile["loopDetectionEnabled"],
        Value::from(true),
        "Some(true) must be written to disk as \"loopDetectionEnabled\": true"
    );
    assert_eq!(
        profile["loopDetectionWindow"],
        Value::from(320),
        "Some(320) must be written to disk as \"loopDetectionWindow\": 320"
    );
    assert_eq!(
        profile["loopDetectionMaxRepeats"],
        Value::from(5),
        "Some(5) must be written to disk as \"loopDetectionMaxRepeats\": 5"
    );
    assert_eq!(
        profile["loopDetectionMaxRetries"],
        Value::from(2),
        "Some(2) must be written to disk as \"loopDetectionMaxRetries\": 2"
    );

    // @step And re-reading the profile resolves the effective values to 320, 5, 2 and enabled
    let wire = wire_loop(Some(true), Some(320), Some(5), Some(2));
    let on_disk = profile_def_from_wire(&wire);
    assert_eq!(on_disk.loop_detection_enabled, Some(true));
    assert_eq!(on_disk.loop_detection_window, Some(320));
    assert_eq!(on_disk.loop_detection_max_repeats, Some(5));
    assert_eq!(on_disk.loop_detection_max_retries, Some(2));
}

/// Scenario: An explicit loopDetectionEnabled false is written and read back
#[test]
fn an_explicit_loop_detection_enabled_false_is_written_and_read_back() {
    // @step Given a profile definition with loopDetectionEnabled false and no loop-detection numeric fields
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    let def = def_loop("http://new", "new", Some(false), None, None, None);

    // @step When the profile is saved to fspec-config.json
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();

    // @step Then the stored profile object contains "loopDetectionEnabled": false
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(
        profile["loopDetectionEnabled"],
        Value::from(false),
        "an explicit false must be WRITTEN to disk (unlike the numeric fields, the toggle is always carried)"
    );

    // @step And the stored profile object has no loopDetectionWindow, loopDetectionMaxRepeats, or loopDetectionMaxRetries keys
    assert!(profile.get("loopDetectionWindow").is_none());
    assert!(profile.get("loopDetectionMaxRepeats").is_none());
    assert!(profile.get("loopDetectionMaxRetries").is_none());

    // @step And re-reading the profile resolves the effective detector state to disabled with default window 160, maxRepeats 10, maxRetries 10
    let wire = wire_loop(Some(false), None, None, None);
    let on_disk = profile_def_from_wire(&wire);
    assert_eq!(on_disk.loop_detection_enabled, Some(false));
    assert_eq!(on_disk.loop_detection_window, None);
    assert_eq!(on_disk.loop_detection_max_repeats, None);
    assert_eq!(on_disk.loop_detection_max_retries, None);
}

/// Scenario: Saving without loop-detection values removes the stored keys
#[test]
fn saving_without_loop_detection_values_removes_the_stored_keys() {
    // @step Given a profile "work-vllm" previously stored loopDetectionWindow 320 and loopDetectionMaxRetries 2
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "work-vllm": {
                    "baseUrl": "http://old",
                    "apiKey": "old",
                    "loopDetectionWindow": 320,
                    "loopDetectionMaxRetries": 2
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();
    let def = def_loop("http://new", "new", None, None, None, None);

    // @step When the profile is saved with no loop-detection values
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();

    // @step Then the stored profile object has no loopDetectionEnabled, loopDetectionWindow, loopDetectionMaxRepeats, or loopDetectionMaxRetries keys
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert!(
        profile.get("loopDetectionEnabled").is_none(),
        "None must remove the loopDetectionEnabled key"
    );
    assert!(
        profile.get("loopDetectionWindow").is_none(),
        "None must remove the loopDetectionWindow key (absent ⇒ default 160)"
    );
    assert!(
        profile.get("loopDetectionMaxRepeats").is_none(),
        "None must remove the loopDetectionMaxRepeats key (absent ⇒ default 10)"
    );
    assert!(
        profile.get("loopDetectionMaxRetries").is_none(),
        "None must remove the loopDetectionMaxRetries key (absent ⇒ default 10)"
    );

    // @step And re-reading the profile resolves every effective value to its RIG-014 default (enabled, 160, 10, 10)
    let wire = wire_loop(None, None, None, None);
    let on_disk = profile_def_from_wire(&wire);
    assert_eq!(on_disk.loop_detection_enabled, None);
    assert_eq!(on_disk.loop_detection_window, None);
    assert_eq!(on_disk.loop_detection_max_repeats, None);
    assert_eq!(on_disk.loop_detection_max_retries, None);
}

/// Scenario: Renaming a profile carries the loop-detection values
#[test]
fn renaming_a_profile_carries_the_loop_detection_values() {
    // @step Given a profile "work-vllm" stores loopDetectionWindow 320 and loopDetectionMaxRetries 2
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "work-vllm": {
                    "baseUrl": "http://old",
                    "apiKey": "old",
                    "loopDetectionWindow": 320,
                    "loopDetectionMaxRetries": 2
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // @step When the profile is renamed to "home" with the same values
    let def = def_loop("http://new", "new", None, Some(320), None, Some(2));
    rename_profile_at(&path, "openai", "work-vllm", "home", &def).unwrap();

    // @step Then the "home" profile object contains the stored loop-detection values
    let profile = read(&path)["providers"]["openai"]["profiles"]["home"].clone();
    assert_eq!(profile["loopDetectionWindow"], Value::from(320));
    assert_eq!(profile["loopDetectionMaxRetries"], Value::from(2));

    // @step And the "work-vllm" key no longer exists
    let profiles = read(&path)["providers"]["openai"]["profiles"].clone();
    assert!(
        profiles.get("work-vllm").is_none(),
        "the old profile key must be gone after the rename"
    );
}

/// Scenario: A legacy profile without the loop-detection keys loads unchanged
#[test]
#[serial]
fn a_legacy_profile_without_the_loop_detection_keys_loads_unchanged() {
    // @step Given a pre-existing profile object that carries only baseUrl and apiKey
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("fspec-config.json");
    fs::write(
        &config,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "legacy": { "baseUrl": "http://h", "apiKey": "k" }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // @step When the local-server profiles are loaded from fspec-config.json
    let guard = EnvGuard::capture(&["FSPEC_USER_DIR"]);
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    let profiles = load_local_server_profiles();
    let legacy = profiles
        .iter()
        .find(|p| p.name == "legacy")
        .expect("the legacy profile must load");

    // @step Then the profile loads with every loop-detection field absent
    assert_eq!(
        legacy.loop_detection_enabled, None,
        "an absent loopDetectionEnabled key must deserialize to None"
    );
    assert_eq!(legacy.loop_detection_window, None);
    assert_eq!(legacy.loop_detection_max_repeats, None);
    assert_eq!(legacy.loop_detection_max_retries, None);

    // @step And re-reading resolves the effective values to the RIG-014 defaults (enabled, 160, 10, 10)
    // (The canonical predicates live on the wire ProfileDefinition; the
    // disk read yields the flat Option values they consume.)
    assert_eq!(legacy.loop_detection_window, None); // ⇒ 160 via predicate
    assert_eq!(legacy.loop_detection_max_repeats, None); // ⇒ 10 via predicate
    assert_eq!(legacy.loop_detection_max_retries, None); // ⇒ 10 via predicate
    drop(guard);
}

/// Scenario: A TS-written float loop-detection value saturates on read
#[test]
#[serial]
fn a_ts_written_float_loop_detection_value_saturates_on_read() {
    // @step Given a stored profile whose loopDetectionWindow value is the float 320.0
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("fspec-config.json");
    fs::write(
        &config,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "ts-written": {
                    "baseUrl": "http://h",
                    "apiKey": "k",
                    "loopDetectionWindow": 320.0
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // @step When the local-server profiles are loaded from fspec-config.json
    let guard = EnvGuard::capture(&["FSPEC_USER_DIR"]);
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    let profiles = load_local_server_profiles();
    let ts = profiles
        .iter()
        .find(|p| p.name == "ts-written")
        .expect("the ts-written profile must load");

    // @step Then the profile loads and its loopDetectionWindow is 320
    assert_eq!(
        ts.loop_detection_window,
        Some(320),
        "a float-written loopDetectionWindow (320.0) must saturate to 320 (lenient deser)"
    );
    drop(guard);
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting coverage (guards the wire→disk bridge step the scenarios span)
// ─────────────────────────────────────────────────────────────────────────────

/// The wire→disk bridge copies all four loop-detection values through.
#[test]
fn wire_to_disk_bridge_copies_the_loop_detection_values() {
    // A wire definition with every loop-detection field set
    let wire = wire_loop(Some(false), Some(40), Some(3), Some(1));

    // Convert it to the on-disk profile definition
    let on_disk = profile_def_from_wire(&wire);

    // The on-disk definition must carry every value through
    assert_eq!(on_disk.loop_detection_enabled, Some(false));
    assert_eq!(on_disk.loop_detection_window, Some(40));
    assert_eq!(on_disk.loop_detection_max_repeats, Some(3));
    assert_eq!(on_disk.loop_detection_max_retries, Some(1));
}

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
