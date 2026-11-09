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
    // MODEL-016: a local-server (openai) profile model's declared thinking
    // levels. Other providers' profiles keep their own behaviour (a Rhai
    // custom provider's level stays unchecked).
    if provider == "openai" {
        if let Some(stored) = profile {
            apply_declared_reasoning(manager, profile_name, model_id, stored)?;
        }
    }
    Ok(())
}

/// MODEL-016: resolve what `model_id` declares on the local-server profile
/// `stored` and record it, with its live check, on the manager. An unusable
/// declaration refuses this model's thinking levels with its reason (other
/// models still resolve); it is logged here and every turn with a chosen
/// level reports it.
fn apply_declared_reasoning(
    manager: &mut ProviderManager,
    profile_name: &str,
    model_id: &str,
    stored: Value,
) -> Result<(), String> {
    let mut profile: crate::profile_sections::LocalServerProfile =
        serde_path_to_error::deserialize(stored).map_err(|error| {
            format!(
                "Invalid saved model configuration: profile '{profile_name}': {}: {}",
                error.path(),
                error.inner()
            )
        })?;
    profile.name = profile_name.to_string();
    let declared = profile.model_reasoning(model_id);
    if let Err(reason) = &declared {
        tracing::warn!(profile = profile_name, model = model_id, %reason, "unusable thinking-level declaration");
    }
    let live_check = match &declared {
        Ok(Some(reasoning)) => codelet_providers::local_reasoning::LiveCheck::for_model(
            &profile.base_url,
            profile.api_key.as_deref(),
            model_id,
            reasoning,
        ),
        Ok(None) | Err(_) => None,
    };
    manager.set_profile_reasoning(declared, live_check);
    Ok(())
}

fn load_profile(provider: &str, name: &str) -> Result<Option<Value>, String> {
    let directory = codelet_common::user_dir::fspec_user_dir()?;
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
