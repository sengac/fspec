use codelet_fspec_tui::transport::BackendError;
use codelet_rpc_types::{ProfileDefinition, ProviderSettingsConfig};

use super::MockBackend;

impl MockBackend {
    pub fn seed_provider_settings(
        &self,
        configs: Vec<ProviderSettingsConfig>,
    ) -> Result<(), BackendError> {
        *self
            .provider_settings
            .lock()
            .map_err(|error| BackendError::ProviderSettings(error.to_string()))? = configs;
        Ok(())
    }

    pub(super) fn provider_settings_snapshot(
        &self,
    ) -> Result<Vec<ProviderSettingsConfig>, BackendError> {
        self.provider_settings
            .lock()
            .map(|configs| configs.clone())
            .map_err(|error| BackendError::ProviderSettings(error.to_string()))
    }

    pub(super) fn store_provider_settings(
        &self,
        provider_id: String,
        settings: ProfileDefinition,
    ) -> Result<(), BackendError> {
        let mut configs = self
            .provider_settings
            .lock()
            .map_err(|error| BackendError::ProviderSettings(error.to_string()))?;
        if let Some(config) = configs
            .iter_mut()
            .find(|config| config.provider_id == provider_id)
        {
            config.settings = settings;
        } else {
            configs.push(ProviderSettingsConfig {
                provider_id,
                settings,
                profiles: Vec::new(),
            });
        }
        Ok(())
    }
}
