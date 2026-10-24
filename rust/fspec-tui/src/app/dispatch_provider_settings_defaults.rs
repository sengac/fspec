use codelet_rpc_types::ProfileDefinition;

use crate::components::Action;

use super::state::App;

impl App {
    pub(crate) fn spawn_provider_settings_read(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            match backend.list_provider_settings().await {
                Ok(configs) => {
                    let _ = action_tx.send(Action::ProviderSettingsConfigsLoaded(configs));
                }
                Err(error) => {
                    tracing::warn!(%error, "list_provider_settings failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✗ settings load failed: {error}"
                    )));
                }
            }
        }));
    }

    pub(crate) fn handle_save_provider_settings(
        &mut self,
        provider_id: String,
        definition: ProfileDefinition,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            match backend
                .save_provider_settings(provider_id.clone(), definition)
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_id} settings saved"
                    )));
                    match backend.list_provider_settings().await {
                        Ok(configs) => {
                            let _ = action_tx.send(Action::ProviderSettingsConfigsLoaded(configs));
                        }
                        Err(error) => {
                            let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                                "✗ settings reload failed: {error}"
                            )));
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, %provider_id, "save_provider_settings failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {error}")));
                }
            }
        }));
    }
}
