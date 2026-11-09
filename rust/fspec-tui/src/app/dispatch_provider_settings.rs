//! App::dispatch routing for the provider-credentials surface
//! that backs the new `ProviderSettingsView` (`/provider` slash command).
//! Introduced: RPC-054.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-049 /
//! RPC-050 / RPC-053 patterns: spawn a tokio task that awaits the
//! backend round-trip, route the response back through the action bus,
//! fold it into the Navigator's `ProviderSettingsView` on the App task.

use codelet_rpc_types::ProviderCredentialInput;
use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-054: open the provider settings view + kick off the initial
    /// list fetch. The Navigator's `apply_action` arm flips
    /// `active_view` to `ProviderSettings` BEFORE this helper runs so
    /// the first render after the dispatch shows the (possibly empty)
    /// view while the backend call resolves.
    pub(crate) fn handle_open_provider_settings_view(&mut self) {
        let view = &mut self.navigator.provider_settings;
        *view = crate::views::ProviderSettingsView::new();
        // BUG-575: one numbered refresh of credentials and settings.
        self.refresh_new_provider_settings_view();
    }

    /// RPC-054: close the provider settings view; the Navigator's
    /// `apply_action` arm flips back to `Agent`.
    pub(crate) fn handle_close_provider_settings_view(&mut self) {
        self.navigator.provider_settings.set_status("");
    }

    /// RPC-054: persist the API key via
    /// `backend.set_provider_credentials` and follow up with a fresh
    /// list refresh so the view repaints with the new state.
    pub(crate) fn handle_save_provider_credentials(
        &mut self,
        provider_id: String,
        api_key: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let provider_for_save = provider_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let creds = ProviderCredentialInput::api_key(api_key);
            match backend
                .set_provider_credentials(provider_for_save.clone(), creds)
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_for_save} credentials saved"
                    )));
                    // BUG-575: shown through the numbered refresh.
                    let _ = action_tx.send(Action::RefreshProviderSettings { navigate: None });
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_for_save, "set_provider_credentials failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054: spawn a delete round-trip and follow up with a fresh
    /// list refresh so the view repaints with the cleared state.
    pub(crate) fn handle_delete_provider_credentials(&mut self, provider_id: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend
                .delete_provider_credentials(provider_id.clone())
                .await
            {
                Ok(()) => {
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✓ {provider_id} credentials cleared"
                    )));
                    // BUG-575: shown through the numbered refresh.
                    let _ = action_tx.send(Action::RefreshProviderSettings { navigate: None });
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_id, "delete_provider_credentials failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054: surface an inline status string on the view.
    pub(crate) fn handle_provider_settings_status(&mut self, status: String) {
        self.navigator.provider_settings.set_status(status);
    }

    /// Route the RPC-054 Action variants through their helpers.
    /// Called from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_provider_settings(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenProviderSettingsView => {
                self.handle_open_provider_settings_view();
            }
            Action::CloseProviderSettingsView => {
                self.handle_close_provider_settings_view();
            }
            Action::SaveProviderCredentials {
                provider_id,
                api_key,
            } => {
                self.handle_save_provider_credentials(provider_id.clone(), api_key.clone());
            }
            Action::ConfirmDeleteProviderCredentials(id) => {
                // RPC-054: the view opens the ConfirmDialog; its Primary
                // acceptance fires this action.
                self.handle_delete_provider_credentials(id.clone());
            }
            Action::ProviderSettingsStatus(s) => {
                self.handle_provider_settings_status(s.clone());
            }
            // PROV-112/113: OAuth disconnect + login flows live in their own
            // sibling module (dispatch_provider_settings_oauth.rs).
            Action::OAuthDisconnect { .. }
            | Action::OAuthLoginStart { .. }
            | Action::OAuthBrowserReady { .. }
            | Action::OAuthHeadlessReady { .. }
            | Action::OAuthDeviceReady { .. }
            | Action::OAuthLoginHeadlessSubmit { .. }
            | Action::OAuthLoginSucceeded { .. }
            | Action::OAuthLoginFailed { .. }
            | Action::OAuthOpenUrl { .. } => {
                return self.try_dispatch_oauth(action);
            }
            // PROV-114: github-copilot device-start (own sibling module).
            Action::OAuthCopilotDeviceStart { .. } => {
                return self.try_dispatch_copilot_oauth(action);
            }
            // PROV-109: profile write actions (SaveProfile / DeleteProfile /
            // ConfirmDeleteProfile) live in the sibling module to keep this
            // file under the 300-LoC ceiling.
            other => return self.try_dispatch_profile_write(other),
        }
        true
    }
}
