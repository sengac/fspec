//! RPC-346 — custom-model persistence (save/delete) on local-server
//! profiles. Exercises the path-injectable helpers against a temp
//! config file so the suite stays fully offline.

use super::config_io::load_local_server_profiles_from;
use super::custom_models::{delete_custom_model_at, save_custom_model_at};
use super::{merge_profile_models, CompactionThreshold, CustomModelDef};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn def(id: &str) -> CustomModelDef {
    CustomModelDef {
        id: id.to_string(),
        display_name: None,
        facade: None,
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        reasoning: None,
        has_vision: None,
        thinking_levels: None,
    }
}

/// Write a config file with one openai profile, returning (dir, path).
fn config_with(profile: &str, custom_models_json: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    let body = format!(
        r#"{{"providers":{{"openai":{{"profiles":{{"{profile}":{{"baseUrl":"http://localhost:8888","apiKey":"sk-test"{custom_models_json}}}}}}}}}}}"#
    );
    fs::write(&path, body).unwrap();
    (dir, path)
}

fn profile_custom_ids(path: &PathBuf, profile: &str) -> Option<Vec<String>> {
    let content = fs::read_to_string(path).unwrap();
    let root: serde_json::Value = serde_json::from_str(&content).unwrap();
    let p = &root["providers"]["openai"]["profiles"][profile];
    p.get("customModels").map(|v| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_string())
            .collect()
    })
}

/// Scenario: Add a custom model to a profile with no custom models
#[test]
fn add_to_profile_without_custom_models() {
    // @step Given an "openai" profile "work-vllm" with no custom models
    let (_dir, path) = config_with("work-vllm", "");

    // @step When I save a new custom model "my-model" in add mode
    save_custom_model_at(&path, "openai", "work-vllm", &def("my-model"), None).unwrap();

    // @step Then the profile's custom models are exactly ["my-model"]
    assert_eq!(
        profile_custom_ids(&path, "work-vllm"),
        Some(vec!["my-model".to_string()])
    );

    // @step And the profile's baseUrl and apiKey are unchanged
    let root: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let p = &root["providers"]["openai"]["profiles"]["work-vllm"];
    assert_eq!(p["baseUrl"], "http://localhost:8888");
    assert_eq!(p["apiKey"], "sk-test");
}

/// Scenario: Edit replaces the matching custom model in place
#[test]
fn edit_replaces_in_place_preserving_order() {
    // @step Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
    let (_dir, path) = config_with(
        "work-vllm",
        r#","customModels":[{"id":"alpha"},{"id":"beta"}]"#,
    );

    // @step When I save a definition with id "alpha2" and original id "alpha"
    save_custom_model_at(&path, "openai", "work-vllm", &def("alpha2"), Some("alpha")).unwrap();

    // @step Then the profile's custom models are exactly ["alpha2", "beta"]
    assert_eq!(
        profile_custom_ids(&path, "work-vllm"),
        Some(vec!["alpha2".to_string(), "beta".to_string()])
    );
}

/// Scenario: Delete one of several custom models
#[test]
fn delete_one_of_several() {
    // @step Given an "openai" profile "work-vllm" with custom models ["alpha", "beta"]
    let (_dir, path) = config_with(
        "work-vllm",
        r#","customModels":[{"id":"alpha"},{"id":"beta"}]"#,
    );

    // @step When I delete the custom model "alpha"
    delete_custom_model_at(&path, "openai", "work-vllm", "alpha").unwrap();

    // @step Then the profile's custom models are exactly ["beta"]
    assert_eq!(
        profile_custom_ids(&path, "work-vllm"),
        Some(vec!["beta".to_string()])
    );
}

/// Scenario: Deleting the last custom model removes the customModels key
#[test]
fn delete_last_removes_key() {
    // @step Given an "openai" profile "work-vllm" with custom models ["alpha"]
    let (_dir, path) = config_with("work-vllm", r#","customModels":[{"id":"alpha"}]"#);

    // @step When I delete the custom model "alpha"
    delete_custom_model_at(&path, "openai", "work-vllm", "alpha").unwrap();

    // @step Then the profile has no customModels key
    assert_eq!(profile_custom_ids(&path, "work-vllm"), None);
}

/// Scenario: Unrelated config is preserved on save
#[test]
fn unrelated_config_is_preserved() {
    // @step Given a config with "openai" profiles "work-vllm" and "home" and a top-level "theme" key
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fspec-config.json");
    fs::write(
        &path,
        r#"{"theme":"dark","providers":{"openai":{"profiles":{"work-vllm":{"baseUrl":"http://a","apiKey":"k1"},"home":{"baseUrl":"http://b","apiKey":"k2"}}}}}"#,
    )
    .unwrap();

    // @step When I save a new custom model "my-model" to "work-vllm" in add mode
    save_custom_model_at(&path, "openai", "work-vllm", &def("my-model"), None).unwrap();

    let root: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    // @step Then the "home" profile is unchanged
    let home = &root["providers"]["openai"]["profiles"]["home"];
    assert_eq!(home["baseUrl"], "http://b");
    assert_eq!(home["apiKey"], "k2");
    assert!(home.get("customModels").is_none());

    // @step And the top-level "theme" key is unchanged
    assert_eq!(root["theme"], "dark");
}

/// Scenario: Saving to a missing profile is refused
#[test]
fn saving_to_missing_profile_is_refused() {
    // @step Given an "openai" profile "work-vllm" exists
    let (_dir, path) = config_with("work-vllm", "");
    let before = fs::read_to_string(&path).unwrap();

    // @step When I save a custom model to the profile "does-not-exist"
    let result = save_custom_model_at(&path, "openai", "does-not-exist", &def("x"), None);

    // @step Then the save returns an error naming the profile "does-not-exist"
    let error = result.expect_err("a missing profile must be refused");
    assert!(error.to_string().contains("does-not-exist"), "{error}");
    // @step And the config file is unchanged
    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(before, after);
}

/// Scenario: Full definition round-trips through save and reload
#[test]
fn full_definition_round_trips() {
    // @step Given an "openai" profile "work-vllm" with no custom models
    let (_dir, path) = config_with("work-vllm", "");

    // @step When I save a custom model with every field set
    let full = CustomModelDef {
        id: "full-model".to_string(),
        display_name: Some("Full Model".to_string()),
        facade: Some("gemini".to_string()),
        context_window: Some(1_048_576),
        max_output_tokens: Some(65_536),
        compaction_threshold: Some(CompactionThreshold {
            threshold_type: "percentage".to_string(),
            value: 80,
        }),
        reasoning: Some(true),
        has_vision: Some(true),
        thinking_levels: None,
    };
    save_custom_model_at(&path, "openai", "work-vllm", &full, None).unwrap();

    // @step And I reload the local-server profiles
    let profiles = load_local_server_profiles_from(&path).expect("the profiles load");

    // @step Then the reloaded profile contains a matching custom model definition
    let profile = profiles.iter().find(|p| p.name == "work-vllm").unwrap();
    assert_eq!(profile.custom_models, vec![full]);
}

/// Scenario: A custom model with a float compaction value still loads
#[test]
fn float_compaction_value_still_loads() {
    // @step Given an "openai" profile "work-vllm" whose custom model stores compactionThreshold.value as the float 80.0
    let (_dir, path) = config_with(
        "work-vllm",
        r#","customModels":[{"id":"m","compactionThreshold":{"type":"percentage","value":80.0}}]"#,
    );

    // @step When I reload the local-server profiles
    let profiles = load_local_server_profiles_from(&path).expect("the profiles load");

    // @step Then the profile loads and the custom model's compaction value is 80
    let profile = profiles.iter().find(|p| p.name == "work-vllm").unwrap();
    let threshold = profile.custom_models[0]
        .compaction_threshold
        .as_ref()
        .unwrap();
    assert_eq!(threshold.value, 80);
}

/// Scenario: A stored display name is surfaced when merging custom models
#[test]
fn stored_display_name_is_surfaced_on_merge() {
    // @step Given a custom model stored with displayName "My Model" and another with no stored display name
    let custom = vec![
        CustomModelDef {
            id: "with-name".to_string(),
            display_name: Some("My Model".to_string()),
            facade: None,
            context_window: None,
            max_output_tokens: None,
            compaction_threshold: None,
            reasoning: Some(true),
            has_vision: None,
            thinking_levels: None,
        },
        CustomModelDef {
            id: "no-name".to_string(),
            display_name: None,
            facade: None,
            context_window: None,
            max_output_tokens: None,
            compaction_threshold: None,
            reasoning: None,
            has_vision: None,
            thinking_levels: None,
        },
    ];

    // @step When the custom models are merged into wire model rows
    let entries = merge_profile_models(Vec::new(), &custom, Some(128_000));

    // @step Then the first row's display name is "My Model" and the second falls back to its id
    let with_name = entries.iter().find(|e| e.id == "with-name").unwrap();
    let no_name = entries.iter().find(|e| e.id == "no-name").unwrap();
    assert_eq!(with_name.display_name, "My Model");
    assert!(with_name.supports_reasoning);
    assert_eq!(no_name.display_name, "no-name");
}
