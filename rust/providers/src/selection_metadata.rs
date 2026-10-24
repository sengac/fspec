use crate::custom::discovery::get_global_providers_dir;
use crate::models::{ModelRegistry, ModelsDevResponse};
use crate::ProviderType;
use serde_json::{json, Value};
use std::path::PathBuf;

pub(crate) struct SelectionMetadata {
    pub catalog_provider: String,
    pub api_model: String,
    pub model_alias: String,
    pub model_options: Option<Value>,
    pub facade: Option<String>,
}

pub(crate) fn catalog_provider_id(provider: &str) -> &str {
    match provider {
        "claude" => "anthropic",
        "gemini" => "google",
        "codex" => "openai",
        "zai" => "z-ai",
        "copilot" => "github-copilot",
        "together" => "togetherai",
        "moonshot" => "moonshotai",
        other => other,
    }
}

pub(crate) fn cached_registry() -> Option<ModelRegistry> {
    let directory = crate::runtime_options::runtime_data_dir()?;
    let data = std::fs::read(directory.join("cache/models.json")).ok()?;
    let response: ModelsDevResponse = serde_json::from_slice(&data).ok()?;
    Some(ModelRegistry::from_response(response))
}

pub(crate) fn selection_metadata(
    provider: &ProviderType,
    provider_id: &str,
    model_id: &str,
    facade_override: Option<&str>,
) -> SelectionMetadata {
    let mut metadata = SelectionMetadata {
        catalog_provider: catalog_provider_id(provider_id).into(),
        api_model: model_id.into(),
        model_alias: model_id.into(),
        model_options: None,
        facade: facade_override.map(String::from),
    };
    if let ProviderType::Custom(name) = provider {
        if let Some(config) = raw_custom_config(name) {
            let model = config
                .get("models")
                .and_then(Value::as_object)
                .and_then(|models| {
                    models.get_key_value(model_id).or_else(|| {
                        let mut matching = models.iter().filter(|(_, model)| {
                            model.get("id").and_then(Value::as_str) == Some(model_id)
                        });
                        let found = matching.next()?;
                        matching.next().is_none().then_some(found)
                    })
                });
            if let Some((alias, model)) = model {
                metadata.model_alias = alias.clone();
                metadata.api_model = model
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string();
                metadata.model_options = Some(json!({
                    "contextWindow":model.get("context_window"),
                    "maxOutputTokens":model.get("max_output_tokens")
                        .or_else(|| config.get("defaults")?.get("max_output_tokens"))
                }));
            }
            metadata.facade = metadata
                .facade
                .or_else(|| crate::custom::derive_facade_for_custom(name));
            if let Some(facade) = metadata.facade.as_deref() {
                metadata.catalog_provider = catalog_provider_id(facade).into();
            }
        }
    }
    metadata
}

fn raw_custom_config(name: &str) -> Option<Value> {
    let mut selected = None;
    for directory in [
        get_global_providers_dir(),
        PathBuf::from(".fspec/providers"),
    ] {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                let config = std::fs::read(entry.path())
                    .ok()
                    .and_then(|data| serde_json::from_slice::<Value>(&data).ok());
                if config
                    .as_ref()
                    .and_then(|config| config.get("name"))
                    .and_then(Value::as_str)
                    == Some(name)
                {
                    selected = config;
                }
            }
        }
    }
    selected
}
