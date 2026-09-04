//! PROV-144 — persistence + wire→disk bridge coverage for the per-profile
//! `maxImages` field. Exercises the read-modify-write save path against a
//! real temp `fspec-config.json`, the `profile_def_from_wire` conversion,
//! and the `LocalServerProfile` disk read-back (lenient deserializer).
//!
//! Feature: spec/features/per-profile-max-images-persistence.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it.
//!
//! RED PHASE: `ProfileDef::max_images` and the `maxImages` merge/read
//! paths do not exist yet, so this target fails to compile until the
//! implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ProfileDefinition;
use codelet_sessions::conversions::profile_def_from_wire;
use codelet_sessions::profile_persistence::{save_profile_at, ProfileDef};
use codelet_sessions::profile_sections::load_local_server_profiles;
use serde_json::{json, Value};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn read(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn def_max_images(base_url: &str, api_key: &str, max_images: Option<u32>) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
        max_images,
    }
}

/// Scenario: The maxImages value round-trips through wire and disk
#[test]
fn the_max_images_value_round_trips_through_wire_and_disk() {
    // @step Given a profile definition with maxImages 7
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    let def = def_max_images("http://new", "new", Some(7));

    // @step When the profile is saved to fspec-config.json
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();

    // @step Then the stored profile object contains "maxImages": 7
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(
        profile["maxImages"],
        Value::from(7),
        "Some(7) must be written to disk as \"maxImages\": 7"
    );

    // @step And re-reading the profile resolves the effective limit to 7
    let wire = ProfileDefinition {
        base_url: "http://new".to_string(),
        api_key: "new".to_string(),
        max_images: Some(7),
        ..ProfileDefinition::default()
    };
    let on_disk = profile_def_from_wire(&wire);
    assert_eq!(on_disk.max_images, Some(7));
}

/// Scenario: A missing maxImages key resolves to the default 4
#[test]
fn a_missing_max_images_key_resolves_to_the_default_4() {
    // @step Given a profile definition without a maxImages field
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    // Seed a profile that previously stored maxImages 2, then save the new
    // definition (no maxImages value) over it.
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "work-vllm": {
                    "baseUrl": "http://old",
                    "apiKey": "old",
                    "maxImages": 2
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();
    let def = def_max_images("http://new", "new", None);

    // @step When the profile is saved to fspec-config.json
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();

    // @step Then the stored profile object has no maxImages key
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert!(
        profile.get("maxImages").is_none(),
        "None must remove the maxImages key (absent ⇒ default 4)"
    );

    // @step And re-reading the profile resolves the effective limit to 4
    let wire = ProfileDefinition {
        base_url: "http://new".to_string(),
        api_key: "new".to_string(),
        max_images: None,
        ..ProfileDefinition::default()
    };
    let on_disk = profile_def_from_wire(&wire);
    assert_eq!(on_disk.max_images, None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting coverage (not 1:1 linked to a single scenario — guards the
// individual plumbing steps the scenario spans)
// ─────────────────────────────────────────────────────────────────────────────

/// The wire→disk bridge copies the maxImages value through the conversion.
#[test]
fn wire_to_disk_bridge_copies_the_max_images_value() {
    // A wire definition with maxImages set to 7
    let wire = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "k".to_string(),
        max_images: Some(7),
        ..ProfileDefinition::default()
    };

    // Convert it to the on-disk profile definition
    let on_disk = profile_def_from_wire(&wire);

    // The on-disk definition must carry maxImages set to 7
    assert_eq!(on_disk.max_images, Some(7));
}

/// The disk read-back tolerates an absent key and a TS-written float value.
#[test]
#[serial]
fn the_disk_read_back_tolerates_absent_and_lenient_max_images() {
    // A stored profile object without a maxImages key
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("fspec-config.json");
    fs::write(
        &config,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "legacy": {
                    "baseUrl": "http://h",
                    "apiKey": "k"
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();

    // Load the profiles from fspec-config.json
    let guard = EnvGuard::capture(&["FSPEC_USER_DIR"]);
    std::env::set_var("FSPEC_USER_DIR", dir.path());
    let profiles = load_local_server_profiles();
    let legacy = profiles
        .iter()
        .find(|p| p.name == "legacy")
        .expect("the legacy profile must load");

    // The profile's maxImages must resolve to absent (⇒ default 4)
    assert_eq!(
        legacy.max_images, None,
        "an absent maxImages key must deserialize to None"
    );

    // Store a TS-written float value (2.0) instead
    fs::write(
        &config,
        serde_json::to_string_pretty(&json!({
            "providers": { "openai": { "profiles": {
                "ts-written": {
                    "baseUrl": "http://h",
                    "apiKey": "k",
                    "maxImages": 2.0
                }
            } } }
        }))
        .unwrap(),
    )
    .unwrap();
    let profiles = load_local_server_profiles();
    let ts = profiles
        .iter()
        .find(|p| p.name == "ts-written")
        .expect("the ts-written profile must load");

    // The profile must survive and the float must saturate to 2 (lenient deser)
    assert_eq!(
        ts.max_images,
        Some(2),
        "a float-written maxImages (2.0) must saturate to 2 (lenient deser)"
    );
    drop(guard);
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
