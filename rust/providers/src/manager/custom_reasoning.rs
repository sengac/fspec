// MODEL-017: a custom provider model's declared thinking levels, read from
// its entry in the provider config. Included by manager.rs.

impl ProviderManager {
    /// MODEL-017: the selected custom provider model's declared reasoning
    /// capability: `Ok(None)` when its config entry declares no
    /// `thinkingLevels` (no reasoning control), an error naming the
    /// provider, the model and the problem when the declaration cannot be
    /// used. Only that model is affected.
    pub fn custom_model_reasoning(
        &self,
    ) -> Result<Option<crate::models::ReasoningCapability>, String> {
        let ProviderType::Custom(provider) = &self.current_provider else {
            return Ok(None);
        };
        let Some(model_id) = self.selected_model.as_deref() else {
            return Ok(None);
        };
        let metadata = crate::selection_metadata::selection_metadata(
            &self.current_provider,
            self.selected_registry_provider_id
                .as_deref()
                .unwrap_or_else(|| self.current_provider.as_str()),
            model_id,
            self.facade_override.as_deref(),
        );
        let Some(raw) = metadata.thinking_levels else {
            return Ok(None);
        };
        let model = metadata.model_alias;
        let declaration: crate::local_reasoning::ThinkingLevelsDeclaration =
            serde_json::from_value(raw).map_err(|error| {
                format!(
                    "custom provider {provider}, model {model}: thinkingLevels cannot be read: \
                     {error}"
                )
            })?;
        declaration.resolve_for_provider(provider, &model).map(Some)
    }
}
