use codelet_providers::ProviderManager;
use serde_json::Value;

pub(super) fn apply_direct_model(
    manager: &mut ProviderManager,
    provider: &str,
    model_id: &str,
) -> Result<(), String> {
    manager
        .set_model_direct(provider, model_id, None, None, None)
        .map_err(|error| format!("Failed to set model: {error}"))
}

pub(super) fn apply_profile_model(
    manager: &mut ProviderManager,
    provider: &str,
    profile_name: &str,
    model_id: &str,
) -> Result<(), String> {
    let profile = load_profile(provider, profile_name)?;
    if profile.is_none() {
        return Err(format!(
            "Cannot resolve saved model profile '{provider}:{profile_name}'"
        ));
    }
    let model = profile.as_ref().and_then(|profile| {
        profile
            .get("customModels")?
            .as_array()?
            .iter()
            .find(|model| model.get("id").and_then(Value::as_str) == Some(model_id))
    });
    let definition = model
        .map(|model| {
            serde_json::from_value::<crate::profile_sections::CustomModelDef>(model.clone())
                .map_err(|error| format!("Invalid saved model configuration: {error}"))
        })
        .transpose()?;
    let facade = definition.as_ref().and_then(|model| model.facade.clone());
    super::model_resolution::apply_profile_env_vars(provider, profile_name, model_id)?;
    manager
        .set_model_direct_with_profile(provider, model_id, Some(profile_name), None, None, facade)
        .map_err(|error| format!("Failed to set profile model: {error}"))?;
    Ok(())
}

fn load_profile(provider: &str, name: &str) -> Result<Option<Value>, String> {
    let Some(directory) = crate::profile_sections::fspec_user_dir() else {
        return Err("Cannot resolve saved model configuration directory".into());
    };
    let content = match std::fs::read(directory.join("fspec-config.json")) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read saved model configuration: {error}")),
    };
    let config: Value = serde_json::from_slice(&content)
        .map_err(|error| format!("Invalid saved model configuration: {error}"))?;
    Ok(crate::provider_settings::provider_config(&config, provider)
        .and_then(|provider| provider.get("profiles"))
        .and_then(|profiles| profiles.get(name))
        .cloned())
}
