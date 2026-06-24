//! Path-injectable unit coverage for the [`super`] read-modify-write cores.
//! Stays offline against a temp `fspec-config.json`. Split into a sibling
//! `#[path]` module to keep `profile_persistence.rs` under the 300-LoC ceiling.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{delete_profile_at, save_profile_at, ProfileDef};
use crate::profile_sections::CompactionThreshold;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn basic(base_url: &str, api_key: &str) -> ProfileDef {
    ProfileDef {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
    }
}

fn write(dir: &TempDir, root: Value) -> PathBuf {
    let path = dir.path().join("fspec-config.json");
    fs::write(&path, serde_json::to_string_pretty(&root).unwrap()).unwrap();
    path
}

fn read(path: &PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn save_creates_missing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    save_profile_at(&path, "openai", "work-vllm", &basic("http://h", "k")).unwrap();
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(profile["baseUrl"], "http://h");
    assert_eq!(profile["apiKey"], "k");
}

#[test]
fn save_preserves_custom_models_and_clears_stale_optional() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://old", "apiKey": "old",
                "contextWindow": 4096, "customModels": [ { "id": "alpha" } ] }
        } } } }),
    );
    // Re-save without contextWindow → it is removed, customModels kept.
    save_profile_at(&path, "openai", "work-vllm", &basic("http://new", "new")).unwrap();
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(profile["baseUrl"], "http://new");
    assert!(profile.get("contextWindow").is_none());
    assert_eq!(profile["customModels"][0]["id"], "alpha");
}

#[test]
fn save_non_openai_is_noop() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k" }
        } } } }),
    );
    let before = fs::read_to_string(&path).unwrap();
    save_profile_at(&path, "anthropic", "work-vllm", &basic("http://x", "y")).unwrap();
    assert_eq!(before, fs::read_to_string(&path).unwrap());
}

#[test]
fn save_writes_compaction_threshold() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    let def = ProfileDef {
        base_url: "http://h".to_string(),
        api_key: "k".to_string(),
        context_window: Some(32_000),
        max_output_tokens: Some(4_096),
        compaction_threshold: Some(CompactionThreshold {
            threshold_type: "percentage".to_string(),
            value: 80,
        }),
    };
    save_profile_at(&path, "openai", "work-vllm", &def).unwrap();
    let profile = read(&path)["providers"]["openai"]["profiles"]["work-vllm"].clone();
    assert_eq!(profile["contextWindow"], 32_000);
    assert_eq!(profile["maxOutputTokens"], 4_096);
    assert_eq!(profile["compactionThreshold"]["type"], "percentage");
    assert_eq!(profile["compactionThreshold"]["value"], 80);
}

#[test]
fn delete_missing_file_is_noop() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    delete_profile_at(&path, "openai", "work-vllm").unwrap();
    assert!(!path.exists());
}

#[test]
fn delete_removes_only_named_profile() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" },
            "home": { "baseUrl": "http://b", "apiKey": "k2" }
        } } } }),
    );
    delete_profile_at(&path, "openai", "work-vllm").unwrap();
    let profiles = read(&path)["providers"]["openai"]["profiles"].clone();
    assert!(profiles.get("work-vllm").is_none());
    assert_eq!(profiles["home"]["apiKey"], "k2");
}

#[test]
fn delete_absent_profile_leaves_file_identical() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        json!({ "providers": { "openai": { "profiles": {
            "work-vllm": { "baseUrl": "http://a", "apiKey": "k1" }
        } } } }),
    );
    let before = fs::read_to_string(&path).unwrap();
    delete_profile_at(&path, "openai", "does-not-exist").unwrap();
    assert_eq!(before, fs::read_to_string(&path).unwrap());
}
