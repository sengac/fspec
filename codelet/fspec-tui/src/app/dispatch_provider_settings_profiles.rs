//! PROV-109: profile write surface for the ProviderSettingsView —
//! `SaveProfile` / `DeleteProfile` / `ConfirmDeleteProfile` dispatch handlers.
//!
//! Split out of `dispatch_provider_settings.rs` to keep that file under the
//! 300-LoC ceiling. Each helper mirrors the RPC-054 save/delete pattern:
//! spawn a tokio task that awaits the PROV-108 backend round-trip, then
//! follow up with a `list_provider_credentials` refresh whose
//! `ProviderCredentialsLoaded` fold reloads the openai profile slice so the
//! view repaints with the new state.

use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-054 / RPC-349 / PROV-111: fold a `list_provider_credentials`
    /// response into the view. The raw list backs the legacy
    /// `visible_providers` path; RPC-349 projects it into the rich NavItem
    /// tree; PROV-100 loads the OpenAI profile display slice; PROV-111 also
    /// loads the FULL per-profile ProfileConfig map so Enter on a Profile row
    /// prefills the EditProfile form. All are small sync local-file reads
    /// (no tokio). Re-loaded on every refresh so a saved/deleted profile's
    /// edit prefill stays in sync — this is the end-to-end repaint path.
    pub(crate) fn handle_provider_credentials_loaded(
        &mut self,
        list: Vec<codelet_rpc_types::ProviderCredentialInfo>,
    ) {
        use crate::views::provider_settings::{profiles_config, projection};
        let profiles = profiles_config::load_openai_profiles();
        let profile_configs = profiles_config::load_openai_profile_configs()
            .into_iter()
            .collect();
        let display = projection::project_display_infos(&list, &profiles);
        let view = &mut self.navigator.provider_settings;
        view.set_providers(list);
        view.set_provider_display_infos(display);
        view.set_profile_configs(profile_configs);
        // PROV-112: after the nav rebuild, honour a pending navigate target
        // (set by the OAuth disconnect dispatch) so the cursor returns to the
        // parent provider row once the Logout row disappears.
        view.apply_pending_navigate();
    }

    /// PROV-109: route the profile write actions. Called from the catch-all
    /// arm of `try_dispatch_provider_settings`. Returns `true` if handled.
    pub(crate) fn try_dispatch_profile_write(&mut self, action: &Action) -> bool {
        match action {
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
            Action::DeleteProfile {
                provider_id,
                profile_name,
            }
            | Action::ConfirmDeleteProfile {
                provider_id,
                profile_name,
            } => {
                // ConfirmDeleteProfile routes through the same backend
                // round-trip as the raw DeleteProfile arm; the view-layer
                // opens the confirm dialog before this fires.
                self.handle_delete_profile(provider_id.clone(), profile_name.clone());
            }
            Action::ProfileDeleteNavigate { provider_id } => {
                // PROV-116: a successful delete records the parent provider so
                // the following reload's `apply_pending_navigate` returns the
                // cursor to the provider row. Set only on success (this action
                // is emitted from the Ok branch), so a failed delete leaves no
                // stale target.
                self.navigator
                    .provider_settings
                    .set_navigate_target(provider_id.clone());
            }
            _ => return false,
        }
        true
    }

    /// PROV-109/PROV-136: persist a profile's connection settings and follow up
    /// with a fresh list refresh so the openai profile slice (reloaded inside
    /// `handle_provider_credentials_loaded`) repaints with the new state.
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
                    if let Ok(list) = backend.list_provider_credentials().await {
                        let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                    }
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
    /// with a fresh list refresh so the removed profile disappears from the
    /// view. Both `DeleteProfile` and `ConfirmDeleteProfile` route here.
    pub(crate) fn handle_delete_profile(&mut self, provider_id: String, profile_name: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend
                .delete_profile(provider_id.clone(), profile_name.clone())
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_id}: {profile_name} profile deleted"
                    )));
                    // PROV-116: record the parent provider BEFORE the reload so
                    // the reload's `apply_pending_navigate` lands the cursor on
                    // the provider row (sent only here, on success).
                    let _ = action_tx.send(Action::ProfileDeleteNavigate {
                        provider_id: provider_id.clone(),
                    });
                    if let Ok(list) = backend.list_provider_credentials().await {
                        let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                    }
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
