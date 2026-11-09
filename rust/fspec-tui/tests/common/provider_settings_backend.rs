use codelet_fspec_tui::transport::BackendError;
use codelet_rpc_types::{ProfileDefinition, ProviderSettingsConfig};

use super::MockBackend;

impl MockBackend {
    pub fn seed_provider_settings(
        &self,
        configs: Vec<ProviderSettingsConfig>,
    ) -> Result<(), BackendError> {
        *self.controls.provider_settings.lock() = configs;
        Ok(())
    }

    /// BUG-575: make every later `list_provider_settings` fail with `message`.
    pub fn fail_list_provider_settings(&self, message: &str) {
        *self.controls.provider_settings_error.lock() = Some(message.to_string());
    }

    /// BUG-575: the stored provider settings, as the backend holds them now.
    pub fn provider_settings_now(&self) -> Vec<ProviderSettingsConfig> {
        self.controls.provider_settings.lock().clone()
    }

    pub(super) fn provider_settings_snapshot(
        &self,
    ) -> Result<Vec<ProviderSettingsConfig>, BackendError> {
        if let Some(message) = self.controls.provider_settings_error.lock().clone() {
            return Err(BackendError::ProviderSettings(message));
        }
        Ok(self.controls.provider_settings.lock().clone())
    }

    pub(super) fn store_provider_settings(
        &self,
        provider_id: String,
        settings: ProfileDefinition,
    ) -> Result<(), BackendError> {
        let mut configs = self.controls.provider_settings.lock();
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
