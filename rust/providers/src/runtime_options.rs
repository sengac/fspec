use crate::ProviderError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRuntimeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_continue: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_thinking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_images: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_detection_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_detection_window: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_detection_max_repeats: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_detection_max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction_threshold: Option<RuntimeCompactionThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCompactionThreshold {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: u64,
}

pub fn resolve_runtime_options(
    config: &Value,
    provider_id: &str,
    profile_name: Option<&str>,
    model_id: Option<&str>,
) -> Result<ProviderRuntimeOptions, ProviderError> {
    resolve_with_model(config, provider_id, profile_name, model_id, None)
}

pub(crate) fn settings_provider_id(provider: &str) -> &str {
    match provider {
        "anthropic" => "claude",
        "google" => "gemini",
        "z-ai" => "zai",
        "copilot" => "github-copilot",
        other => other,
    }
}

pub(crate) fn load_runtime_options(
    provider: &str,
    profile: Option<&str>,
    model: Option<&str>,
    custom_model: Option<&Value>,
) -> Result<ProviderRuntimeOptions, ProviderError> {
    let config = if let Some(directory) = runtime_data_dir() {
        let current = std::env::current_dir()
            .map_err(|error| ProviderError::config(provider, error.to_string()))?;
        codelet_common::fspec_config::load_config_with_dirs(&directory, &current)
            .map_err(|error| ProviderError::config(provider, error))?
    } else {
        Value::Null
    };
    resolve_with_model(&config, provider, profile, model, custom_model)
}

pub(crate) fn runtime_data_dir() -> Option<PathBuf> {
    std::env::var_os("FSPEC_USER_DIR")
        .map(PathBuf::from)
        .or_else(|| codelet_common::get_data_dir().ok())
}

fn resolve_with_model(
    config: &Value,
    provider_id: &str,
    profile_name: Option<&str>,
    model_id: Option<&str>,
    custom_model: Option<&Value>,
) -> Result<ProviderRuntimeOptions, ProviderError> {
    let providers = config.get("providers");
    let provider = providers.and_then(|providers| {
        providers
            .get(provider_id)
            .or_else(|| providers.get(settings_provider_id(provider_id)))
            .or_else(|| {
                let alias = match provider_id {
                    "claude" => "anthropic",
                    "gemini" => "google",
                    "zai" => "z-ai",
                    "github-copilot" => "copilot",
                    _ => return None,
                };
                providers.get(alias)
            })
    });
    let profile = profile_name.and_then(|name| provider?.get("profiles")?.get(name));
    let mut merged = serde_json::Map::new();
    for layer in [
        provider.and_then(|provider| provider.get("settings")),
        profile,
        profile.and_then(|profile| profile.get("settings")),
        custom_model,
        matching_model(provider, model_id),
        matching_model(profile, model_id),
    ]
    .into_iter()
    .flatten()
    {
        for (key, value) in layer.as_object().into_iter().flatten() {
            if !value.is_null() {
                merged.insert(key.clone(), value.clone());
            }
        }
    }
    let options: ProviderRuntimeOptions =
        serde_json::from_value(Value::Object(merged)).map_err(|error| {
            ProviderError::config(provider_id, format!("Invalid runtime settings: {error}"))
        })?;
    validate(&options, provider_id)?;
    Ok(options)
}

fn matching_model<'config>(
    owner: Option<&'config Value>,
    model: Option<&str>,
) -> Option<&'config Value> {
    let model = model?;
    owner?
        .get("customModels")?
        .as_array()?
        .iter()
        .find(|definition| definition.get("id").and_then(Value::as_str) == Some(model))
}

fn validate(options: &ProviderRuntimeOptions, provider: &str) -> Result<(), ProviderError> {
    if options.context_window == Some(0) || options.max_output_tokens == Some(0) {
        return Err(ProviderError::config(
            provider,
            "Token overrides must be positive",
        ));
    }
    if let Some(threshold) = &options.compaction_threshold {
        let valid = match threshold.kind.as_str() {
            "tokens" => threshold.value > 0,
            "percentage" => (1..=100).contains(&threshold.value),
            _ => false,
        };
        if !valid {
            return Err(ProviderError::config(
                provider,
                "Invalid compaction threshold",
            ));
        }
    }
    Ok(())
}
