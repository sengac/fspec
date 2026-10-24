use crate::custom::config::ProviderConfig;
use crate::custom::error::CustomProviderError;

pub(super) fn resolve_model_alias(
    config: &ProviderConfig,
    selection: String,
) -> Result<String, CustomProviderError> {
    if config.models.contains_key(&selection) {
        return Ok(selection);
    }
    let mut matching = config
        .models
        .iter()
        .filter(|(_, model)| model.id == selection);
    let (alias, _) = matching.next().ok_or_else(|| {
        CustomProviderError::RhaiRuntimeError(format!(
            "model alias '{selection}' not found in config.models"
        ))
    })?;
    if matching.next().is_some() {
        return Err(CustomProviderError::RhaiRuntimeError(format!(
            "model ID '{selection}' matches multiple aliases in config.models"
        )));
    }
    Ok(alias.clone())
}
