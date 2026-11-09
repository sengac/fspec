use codelet_rpc_types::ProfileDefinition;

use crate::components::Action;

use super::state::App;

impl App {
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
                    // BUG-575: shown through the numbered refresh.
                    let _ = action_tx.send(Action::RefreshProviderSettings { navigate: None });
                }
                Err(error) => {
                    tracing::warn!(%error, %provider_id, "save_provider_settings failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {error}")));
                }
            }
        }));
    }
}
