//! Feature: spec/features/provider-settings-backend-parity.feature

use std::error::Error;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{ModelEntry, ProfileDefinition, ProviderInfo};
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use serial_test::serial;

#[path = "common/profile_rpc_helpers.rs"]
mod helpers;

fn runtime_definition() -> ProfileDefinition {
    ProfileDefinition {
        context_window: Some(256_000),
        max_output_tokens: Some(32_000),
        compaction_threshold_type: Some("percentage".into()),
        compaction_threshold_value: Some(75),
        streaming: Some(false),
        auto_continue: Some(3),
        preserve_thinking: Some(true),
        max_images: Some(0),
        loop_detection_enabled: Some(false),
        loop_detection_window: Some(240),
        loop_detection_max_repeats: Some(8),
        loop_detection_max_retries: Some(0),
        ..ProfileDefinition::default()
    }
}

#[test]
#[serial]
fn provider_settings_round_trip_without_changing_credentials() -> Result<(), Box<dyn Error>> {
    // @step Given saved built-in and custom provider credentials and unrelated configuration
    let directory = setup_temp_directory()?;
    let _environment = helpers::point_env(directory.path());
    let config_path = directory.path().join("fspec-config.json");
    let original = json!({
        "theme": "dark",
        "providers": {
            "codex": {"oauth": {"accessToken": "codex-token"}, "endpoint": "unchanged"},
            "claude": {"apiKey": "anthropic-key"},
            "private-facade": {"apiKey": "custom-key", "facade": "openai"}
        }
    });
    std::fs::write(&config_path, serde_json::to_vec(&original)?)?;
    let credential_path = directory.path().join("credentials.json");
    let credential_bytes = br#"{"codex":{"access_token":"oauth-secret"}}"#;
    std::fs::write(&credential_path, credential_bytes)?;
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(SessionManager::new());
    let definition = runtime_definition();

    // @step When shared runtime defaults are saved and read through the backend
    for provider_id in ["codex", "anthropic", "private-facade"] {
        handle.save_provider_settings(provider_id, &definition)?;
    }
    let loaded = handle.list_provider_settings()?;

    // @step Then explicit settings round trip while credentials and unrelated configuration remain unchanged
    let persisted: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    assert_eq!(persisted["theme"], original["theme"]);
    assert_eq!(std::fs::read(&credential_path)?, credential_bytes);
    assert!(
        persisted["providers"].get("anthropic").is_none(),
        "canonical saves must update the existing legacy alias, not create a shadow provider"
    );
    for provider_id in ["codex", "anthropic", "private-facade"] {
        let settings = loaded
            .iter()
            .find(|entry| entry.provider_id == provider_id)
            .ok_or("saved provider settings missing from backend read")?;
        assert_eq!(settings.settings, definition);
        let storage_id = if provider_id == "anthropic" {
            "claude"
        } else {
            provider_id
        };
        let mut provider = persisted["providers"][storage_id].clone();
        let saved_settings = provider
            .as_object_mut()
            .ok_or("provider must remain an object")?
            .remove("settings")
            .ok_or("provider defaults must be nested under settings")?;
        assert_eq!(provider, original["providers"][storage_id]);
        assert_eq!(saved_settings["contextWindow"], 256_000);
        assert_eq!(saved_settings["maxOutputTokens"], 32_000);
        assert_eq!(saved_settings["streaming"], false);
        assert_eq!(saved_settings["maxImages"], 0);
        assert!(saved_settings.get("apiKey").is_none());
        assert!(saved_settings.get("baseUrl").is_none());
    }
    Ok(())
}

#[test]
#[serial]
fn named_profile_crud_is_provider_scoped() -> Result<(), Box<dyn Error>> {
    // @step Given Codex and Claude each have a profile named careful
    let directory = setup_temp_directory()?;
    let _environment = helpers::point_env(directory.path());
    let config_path = directory.path().join("fspec-config.json");
    let original = json!({"providers": {
        "codex": {"oauth": {"accessToken": "codex-token"}, "profiles": {
            "careful": {"contextWindow": 120_000, "customModels": [{"id": "gpt-test"}]}
        }},
        "claude": {"oauth": {"accessToken": "claude-token"}, "profiles": {
            "careful": {"contextWindow": 220_000, "preserveThinking": true}
        }}
    }});
    std::fs::write(&config_path, serde_json::to_vec(&original)?)?;
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(SessionManager::new());
    let definition = runtime_definition();

    // @step When the Codex profile is renamed and deleted through the backend
    handle.rename_profile("codex", "careful", "renamed", &definition)?;
    let after_rename: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    assert!(after_rename["providers"]["codex"]["profiles"]
        .get("careful")
        .is_none());
    assert_eq!(
        after_rename["providers"]["codex"]["profiles"]["renamed"]["customModels"],
        original["providers"]["codex"]["profiles"]["careful"]["customModels"]
    );
    handle.delete_profile("codex", "renamed")?;
    handle.save_profile("anthropic", "temporary", &definition)?;
    let after_alias_save: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    assert!(after_alias_save["providers"].get("anthropic").is_none());
    assert!(after_alias_save["providers"]["claude"]["profiles"]
        .get("temporary")
        .is_some());
    handle.rename_profile("anthropic", "temporary", "temporary-renamed", &definition)?;
    let after_alias_rename: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    assert!(after_alias_rename["providers"]["claude"]["profiles"]
        .get("temporary")
        .is_none());
    assert!(after_alias_rename["providers"]["claude"]["profiles"]
        .get("temporary-renamed")
        .is_some());
    handle.delete_profile("anthropic", "temporary-renamed")?;
    let loaded = handle.list_provider_settings()?;

    // @step Then the Claude profile and its inherited authentication remain unchanged
    let persisted: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    assert_eq!(
        persisted["providers"]["claude"],
        original["providers"]["claude"]
    );
    assert_eq!(
        persisted["providers"]["codex"]["oauth"],
        original["providers"]["codex"]["oauth"]
    );
    assert!(persisted["providers"]["codex"]["profiles"]
        .get("renamed")
        .is_none());
    let claude = loaded
        .iter()
        .find(|entry| entry.provider_id == "anthropic")
        .ok_or("Claude settings missing")?;
    let careful = claude
        .profiles
        .iter()
        .find(|profile| profile.name == "careful")
        .ok_or("Claude profile was removed by Codex CRUD")?;
    assert_eq!(careful.definition.context_window, Some(220_000));
    assert!(careful.definition.api_key.is_empty());
    assert!(careful.definition.base_url.is_empty());
    Ok(())
}

#[test]
#[serial]
fn legacy_openai_profiles_remain_readable() -> Result<(), Box<dyn Error>> {
    // @step Given an existing OpenAI profile with connection fields and runtime overrides
    let directory = setup_temp_directory()?;
    let _environment = helpers::point_env(directory.path());
    let config_path = directory.path().join("fspec-config.json");
    let original = json!({"providers": {"openai": {"profiles": {"legacy": {
        "baseUrl": "http://127.0.0.1:8123/v1", "apiKey": "legacy-key",
        "contextWindow": 64_000, "maxOutputTokens": 8_000,
        "compactionThreshold": {"type": "percentage", "value": 80},
        "streaming": false, "autoContinue": 2,
        "customModels": [{"id": "private-model", "contextWindow": 96_000}],
        "unknownExtension": {"keep": true}
    }}}}});
    std::fs::write(&config_path, serde_json::to_vec(&original)?)?;
    let handle: Arc<dyn SessionManagerHandle> = Arc::new(SessionManager::new());

    // @step When provider settings are read and the profile is edited
    let loaded = handle.list_provider_settings()?;
    let openai = loaded
        .iter()
        .find(|entry| entry.provider_id == "openai")
        .ok_or("OpenAI settings missing")?;
    let legacy = openai
        .profiles
        .iter()
        .find(|profile| profile.name == "legacy")
        .ok_or("legacy OpenAI profile missing")?;
    let mut definition = legacy.definition.clone();
    assert_eq!(definition.base_url, "http://127.0.0.1:8123/v1");
    assert_eq!(definition.api_key, "legacy-key");
    assert_eq!(definition.context_window, Some(64_000));
    assert_eq!(definition.max_output_tokens, Some(8_000));
    assert_eq!(
        definition.compaction_threshold_type.as_deref(),
        Some("percentage")
    );
    assert_eq!(definition.compaction_threshold_value, Some(80));
    assert_eq!(definition.streaming, Some(false));
    definition.auto_continue = Some(4);
    handle.save_profile("openai", "legacy", &definition)?;

    // @step Then legacy values and unrelated profile fields are preserved
    let persisted: Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    let mut expected = original;
    expected["providers"]["openai"]["profiles"]["legacy"]["autoContinue"] = json!(4);
    assert_eq!(persisted, expected);
    Ok(())
}

#[test]
#[serial]
fn inherited_authentication_profiles_are_selectable() -> Result<(), Box<dyn Error>> {
    // @step Given a non-OpenAI provider has a named runtime profile without a discovery endpoint
    let directory = setup_temp_directory()?;
    let _environment = helpers::point_env(directory.path());
    std::fs::write(
        directory.path().join("fspec-config.json"),
        serde_json::to_vec(&json!({"providers": {"claude": {"profiles": {
            "careful": {"contextWindow": 220_000, "preserveThinking": true}
        }}}}))?,
    )?;
    let models = vec![ModelEntry {
        id: "claude-test".into(),
        display_name: "Claude test".into(),
        context_window: 200_000,
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }];
    let provider = ProviderInfo {
        key: "anthropic".into(),
        display_name: "Claude".into(),
        models: models.clone(),
        profile_name: None,
        is_unreachable: false,
    };

    // @step When available model sections are loaded
    let sections =
        codelet_sessions::profile_sections::build_inherited_profile_sections(&[provider]);

    // @step Then the profile exposes the provider models without requiring an OpenAI discovery probe
    assert_eq!(
        sections.len(),
        1,
        "provider aliases must not duplicate profiles"
    );
    let profile = sections
        .iter()
        .find(|section| section.key == "anthropic:careful")
        .ok_or("inherited-auth profile missing from model sections")?;
    assert_eq!(profile.profile_name.as_deref(), Some("careful"));
    assert!(!profile.is_unreachable);
    assert_eq!(profile.models.len(), models.len());
    assert_eq!(profile.models[0].id, models[0].id);
    assert_eq!(profile.models[0].context_window, models[0].context_window);
    Ok(())
}
