use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use codelet_providers::ProviderRuntimeOptions;
use codelet_rpc_types::{ProfileDefinition, ProviderProfileConfig, ProviderSettingsConfig};
use serde_json::{Map, Value};

use crate::profile_sections::{fspec_user_dir, write_config_value};

fn config_path() -> io::Result<PathBuf> {
    fspec_user_dir()
        .map(|directory| directory.join("fspec-config.json"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Cannot resolve fspec user directory",
            )
        })
}

pub(crate) fn canonical_provider_id(provider: &str) -> &str {
    match provider {
        "claude" => "anthropic",
        "google" => "gemini",
        "z-ai" => "zai",
        "copilot" => "github-copilot",
        _ => provider,
    }
}

pub(crate) fn provider_config<'config>(
    root: &'config Value,
    provider: &str,
) -> Option<&'config Value> {
    let storage_id = provider_storage_id(root, provider);
    root.get("providers")
        .and_then(|providers| providers.get(&storage_id))
}

pub(crate) fn provider_storage_id(root: &Value, provider: &str) -> String {
    let alias = match provider {
        "anthropic" => "claude",
        "claude" => "anthropic",
        "google" => "gemini",
        "gemini" => "google",
        "z-ai" => "zai",
        "zai" => "z-ai",
        "copilot" => "github-copilot",
        "github-copilot" => "copilot",
        _ => provider,
    };
    let providers = root.get("providers");
    if providers
        .and_then(|providers| providers.get(provider))
        .is_some()
    {
        provider.to_string()
    } else if providers
        .and_then(|providers| providers.get(alias))
        .is_some()
    {
        alias.to_string()
    } else {
        canonical_provider_id(provider).to_string()
    }
}

fn read_config(path: &Path) -> io::Result<Value> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes)?;
            if !value.is_object() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Provider configuration must be an object",
                ));
            }
            Ok(value)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(error) => Err(error),
    }
}

pub fn list_provider_settings() -> io::Result<Vec<ProviderSettingsConfig>> {
    list_provider_settings_at(&config_path()?)
}

pub fn list_provider_settings_at(path: &Path) -> io::Result<Vec<ProviderSettingsConfig>> {
    let root = read_config(path)?;
    let providers = root.get("providers").and_then(Value::as_object);
    let mut identifiers: BTreeSet<String> = codelet_providers::catalog::CANONICAL_PROVIDERS
        .iter()
        .map(|provider| provider.id.to_string())
        .collect();
    let discovered = codelet_providers::custom::list_providers_info().map_err(io::Error::other)?;
    identifiers.extend(
        discovered
            .iter()
            .map(|provider| canonical_provider_id(&provider.name).to_string()),
    );
    if let Some(providers) = providers {
        identifiers.extend(
            providers
                .keys()
                .map(|provider| canonical_provider_id(provider).to_string()),
        );
    }
    identifiers
        .into_iter()
        .map(|provider_id| {
            let provider = provider_config(&root, &provider_id);
            let settings =
                definition_from_value(provider.and_then(|provider| provider.get("settings")))?;
            let mut profiles = provider
                .and_then(|provider| provider.get("profiles"))
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|profiles| profiles.iter())
                .map(|(name, value)| {
                    Ok(ProviderProfileConfig {
                        name: name.clone(),
                        definition: definition_from_value(Some(value))?,
                    })
                })
                .collect::<io::Result<Vec<_>>>()?;
            profiles.sort_by(|left, right| left.name.cmp(&right.name));
            Ok(ProviderSettingsConfig {
                provider_id,
                settings,
                profiles,
            })
        })
        .collect()
}

pub fn save_provider_settings(provider_id: &str, definition: &ProfileDefinition) -> io::Result<()> {
    save_provider_settings_at(&config_path()?, provider_id, definition)
}

pub fn save_provider_settings_at(
    path: &Path,
    provider_id: &str,
    definition: &ProfileDefinition,
) -> io::Result<()> {
    if provider_id.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Provider identifier must not be empty",
        ));
    }
    validate_definition(definition)?;
    let mut root = read_config(path)?;
    let storage_id = provider_storage_id(&root, provider_id);
    let providers = object_child(&mut root, "providers")?;
    let provider = object_child(providers, &storage_id)?;
    let settings = object_child(provider, "settings")?;
    let object = settings
        .as_object_mut()
        .ok_or_else(|| io::Error::other("Settings must be an object"))?;
    let definition = crate::conversions::profile_def_from_wire(definition);
    crate::profile_persistence::merge_profile(object, &definition);
    object.remove("apiKey");
    object.remove("baseUrl");
    let _: ProviderRuntimeOptions = serde_json::from_value(settings.clone())?;
    write_config_value(path, &root)
}

fn object_child<'a>(parent: &'a mut Value, key: &str) -> io::Result<&'a mut Value> {
    let object = parent.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Configuration must be an object",
        )
    })?;
    let value = object
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()));
    if !value.is_object() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Configuration field '{key}' must be an object"),
        ));
    }
    Ok(value)
}

pub(crate) fn validate_definition(definition: &ProfileDefinition) -> io::Result<()> {
    if definition.context_window == Some(0)
        || definition.max_output_tokens == Some(0)
        || definition.loop_detection_window == Some(0)
        || definition.loop_detection_max_repeats == Some(0)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Token limits and loop window/repeat limits must be positive",
        ));
    }
    match (
        definition.compaction_threshold_type.as_deref(),
        definition.compaction_threshold_value,
    ) {
        (None, None) => Ok(()),
        (Some("tokens"), Some(value)) if value > 0 => Ok(()),
        (Some("percentage"), Some(1..=100)) => Ok(()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Compaction threshold requires positive tokens or percentage 1-100",
        )),
    }
}

fn definition_from_value(value: Option<&Value>) -> io::Result<ProfileDefinition> {
    let value = value.cloned().unwrap_or_else(|| Value::Object(Map::new()));
    let options: ProviderRuntimeOptions = serde_json::from_value(value.clone())?;
    let threshold = serde_json::to_value(&options)?
        .get("compactionThreshold")
        .cloned();
    Ok(ProfileDefinition {
        base_url: value
            .get("baseUrl")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        api_key: value
            .get("apiKey")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        context_window: options
            .context_window
            .map(u32::try_from)
            .transpose()
            .map_err(io::Error::other)?,
        max_output_tokens: options
            .max_output_tokens
            .map(u32::try_from)
            .transpose()
            .map_err(io::Error::other)?,
        compaction_threshold_type: threshold
            .as_ref()
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            .map(str::to_string),
        compaction_threshold_value: threshold
            .as_ref()
            .and_then(|value| value.get("value"))
            .and_then(Value::as_u64)
            .map(u32::try_from)
            .transpose()
            .map_err(io::Error::other)?,
        streaming: options.streaming,
        auto_continue: options.auto_continue,
        preserve_thinking: options.preserve_thinking,
        max_images: options.max_images,
        loop_detection_enabled: options.loop_detection_enabled,
        loop_detection_window: options.loop_detection_window,
        loop_detection_max_repeats: options.loop_detection_max_repeats,
        loop_detection_max_retries: options.loop_detection_max_retries,
    })
}
