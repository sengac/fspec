//! Provider profile writes and the numbered settings refresh dispatch (BUG-575).

use tokio::task::JoinHandle;

use crate::components::Action;
use crate::views::provider_settings::refresh::NavigateTarget;

use super::state::App;

impl App {
    /// PROV-109: route the profile write actions. Called from the catch-all
    /// arm of `try_dispatch_provider_settings`. Returns `true` if handled.
    pub(crate) fn try_dispatch_profile_write(&mut self, action: &Action) -> bool {
        match action {
            Action::RefreshProviderSettings { navigate } => {
                self.spawn_provider_settings_refresh(navigate.clone());
            }
            Action::ProviderSettingsRefreshed(refresh) => {
                self.navigator
                    .provider_settings
                    .apply_refresh((**refresh).clone());
            }
            Action::SaveProviderSettings {
                provider_id,
                definition,
            } => {
                self.handle_save_provider_settings(provider_id.clone(), definition.clone());
            }
            Action::SaveProfile {
                provider_id,
                profile_name,
                old_profile_name,
                definition,
            } => {
                self.handle_save_profile(
                    provider_id.clone(),
                    profile_name.clone(),
                    old_profile_name.clone(),
                    definition.clone(),
                );
            }
            Action::ConfirmDeleteProfile {
                provider_id,
                profile_name,
            } => {
                // The view opens the confirm dialog before this fires.
                self.handle_delete_profile(provider_id.clone(), profile_name.clone());
            }
            _ => return false,
        }
        true
    }

    /// PROV-109/PROV-136: persist a profile's connection settings and follow up
    /// with a numbered refresh (BUG-575) that keeps the cursor on the saved
    /// profile, under its new name after a rename.
    /// When `old_profile_name` is `Some(old)` and `old != profile_name` this is
    /// an edit-mode RENAME: it routes through `backend.rename_profile` (which
    /// moves the old key to the new name, preserving customModels, and rejects a
    /// collision). Otherwise it is a plain save. Mirrors
    /// `handle_save_provider_credentials`.
    pub(crate) fn handle_save_profile(
        &mut self,
        provider_id: String,
        profile_name: String,
        old_profile_name: Option<String>,
        definition: codelet_rpc_types::ProfileDefinition,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let navigate = Some(self.navigate_request(NavigateTarget::Profile {
            provider_id: provider_id.clone(),
            name: profile_name.clone(),
        }));
        let rename_from = match old_profile_name {
            Some(old) if old != profile_name => Some(old),
            _ => None,
        };
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let result = match &rename_from {
                Some(old) => {
                    backend
                        .rename_profile(
                            provider_id.clone(),
                            old.clone(),
                            profile_name.clone(),
                            definition,
                        )
                        .await
                }
                None => {
                    backend
                        .save_profile(provider_id.clone(), profile_name.clone(), definition)
                        .await
                }
            };
            match result {
                Ok(()) => {
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_id}: {profile_name} profile saved"
                    )));
                    let _ = action_tx.send(Action::RefreshProviderSettings { navigate });
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_id, profile = %profile_name, "save_profile failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-109: delete a profile via `backend.delete_profile` and follow up
    /// with a numbered refresh so the removed profile disappears from the
    /// view. `ConfirmDeleteProfile` routes here.
    pub(crate) fn handle_delete_profile(&mut self, provider_id: String, profile_name: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        // PROV-116 / BUG-575: a successful delete returns the cursor to the
        // provider row once its own refresh is shown (a failed delete sends
        // no refresh, so the cursor never jumps).
        let navigate = Some(self.navigate_request(NavigateTarget::Provider(provider_id.clone())));
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            // BUG-613: the daemon clears a default model of the deleted profile.
            let before = backend.get_default_model().await;
            match backend
                .delete_profile(provider_id.clone(), profile_name.clone())
                .await
            {
                Ok(()) => {
                    let cleared = match (before, backend.get_default_model().await) {
                        (Ok(before), Ok(after)) => {
                            super::dispatch_model_selector::default_cleared_notice(
                                before.as_deref(),
                                after.as_deref(),
                            )
                            .map(|notice| format!(" — {}", notice.trim_start_matches("[info] ")))
                        }
                        (Err(error), _) | (_, Err(error)) => {
                            Some(format!(" — the default model could not be read: {error:#}"))
                        }
                    };
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_id}: {profile_name} profile deleted{}",
                        cleared.unwrap_or_default()
                    )));
                    let _ = action_tx.send(Action::RefreshProviderSettings { navigate });
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_id, profile = %profile_name, "delete_profile failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }
}
