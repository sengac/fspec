use super::{ProviderManager, ProviderType};
use crate::models::ModelInfo;
use crate::runtime_options::load_runtime_options;
use crate::selection_metadata::{
    cached_registry, catalog_provider_id, selection_metadata, SelectionMetadata,
};
use crate::{ProviderError, ProviderRuntimeOptions};

pub(super) fn apply_script_limits(
    provider: &ProviderType,
    metadata: &SelectionMetadata,
    options: &mut ProviderRuntimeOptions,
) -> Option<(String, u64)> {
    let ProviderType::Custom(name) = provider else {
        return None;
    };
    let limits = crate::custom::lookup_script_model_limits(name, &metadata.model_alias);
    options.context_window = limits.context_window.or(options.context_window);
    options.max_output_tokens = limits.max_output_tokens.or(options.max_output_tokens);
    limits.compaction_threshold
}

impl ProviderManager {
    pub fn runtime_options(&self) -> &ProviderRuntimeOptions {
        &self.runtime_options
    }

    pub fn should_send_max_output_tokens(&self) -> bool {
        self.current_provider != ProviderType::Codex
            && self.facade_override.as_deref() != Some("codex")
    }

    pub fn reload_runtime_settings(&mut self) -> Result<(), ProviderError> {
        let metadata = self.runtime_selection_metadata();
        let mut options = load_runtime_options(
            self.current_provider.as_str(),
            self.selected_profile_name.as_deref(),
            metadata
                .as_ref()
                .map(|metadata| metadata.api_model.as_str()),
            metadata
                .as_ref()
                .and_then(|metadata| metadata.model_options.as_ref()),
        )?;
        if let Some(metadata) = &metadata {
            apply_script_limits(&self.current_provider, metadata, &mut options);
        }
        self.runtime_options = options;
        Ok(())
    }

    fn runtime_selection_metadata(&self) -> Option<SelectionMetadata> {
        self.selected_model.as_deref().map(|model| {
            selection_metadata(
                &self.current_provider,
                self.selected_registry_provider_id
                    .as_deref()
                    .unwrap_or_else(|| self.current_provider.as_str()),
                model,
                self.facade_override.as_deref(),
            )
        })
    }

    pub(super) fn refresh_runtime_options(&mut self) -> Result<(), ProviderError> {
        if self.model_registry.is_none() {
            self.model_registry = cached_registry();
        }
        let metadata = self.runtime_selection_metadata();
        let mut options = load_runtime_options(
            self.current_provider.as_str(),
            self.selected_profile_name.as_deref(),
            metadata
                .as_ref()
                .map(|metadata| metadata.api_model.as_str()),
            metadata
                .as_ref()
                .and_then(|metadata| metadata.model_options.as_ref()),
        )?;
        let limits = self
            .selection_model_info()
            .map(|model| (model.limit.context as usize, model.limit.output as usize));
        self.registry_context_window = limits
            .map(|(context, _)| context)
            .filter(|value| *value > 0);
        self.registry_max_output_tokens =
            limits.map(|(_, output)| output).filter(|value| *value > 0);
        self.compaction_threshold_override = metadata.as_ref().and_then(|metadata| {
            apply_script_limits(&self.current_provider, metadata, &mut options)
        });
        self.runtime_options = options;
        if let Some(metadata) = metadata {
            self.facade_override = metadata.facade;
        }
        Ok(())
    }

    pub(super) fn selection_model_info(&self) -> Option<&ModelInfo> {
        let registry = self.model_registry.as_ref()?;
        let provider = self.selected_registry_provider_id.as_deref()?;
        let model = self.selected_model.as_deref()?;
        let metadata = selection_metadata(
            &self.current_provider,
            provider,
            model,
            self.facade_override.as_deref(),
        );
        registry
            .get_model(catalog_provider_id(provider), &metadata.api_model)
            .or_else(|_| registry.get_model(provider, &metadata.api_model))
            .or_else(|_| registry.get_model(&metadata.catalog_provider, &metadata.api_model))
            .ok()
    }
}
