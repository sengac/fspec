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
        // Reset to a clean list-mode view so a previous session's edit
        // state never leaks back in.
        self.navigator.provider_settings = crate::views::ProviderSettingsView::new();
        self.spawn_list_provider_credentials();
    }

    /// RPC-054: close the provider settings view; the Navigator's
    /// `apply_action` arm flips back to `Agent`.
    pub(crate) fn handle_close_provider_settings_view(&mut self) {
        self.navigator.provider_settings.set_status("");
    }

    /// RPC-054: spawn `backend.list_provider_credentials()` and route
    /// the result into the view via `Action::ProviderCredentialsLoaded`.
    fn spawn_list_provider_credentials(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.list_provider_credentials().await {
                Ok(list) => {
                    let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "list_provider_credentials failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!(
                        "✗ list failed: {e}"
                    )));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054 / RPC-349: fold a `list_provider_credentials` response into
    /// the view. The raw list still backs the legacy `visible_providers`
    /// path (delete-focus, `d` keybind), but RPC-349 additionally projects
    /// it into `ProviderDisplayInfo`s and feeds the rich RPC-103 NavItem
    /// tree via `set_provider_display_infos` — otherwise `nav_items` stays
    /// empty and the screen falls back to the legacy flat list (the bug
    /// this card fixes). `openai_profiles` is empty until a list-profiles
    /// RPC exists; the trailing "Add Profile" row still renders.
    pub(crate) fn handle_provider_credentials_loaded(
        &mut self,
        list: Vec<codelet_rpc_types::ProviderCredentialInfo>,
    ) {
        let display =
            crate::views::provider_settings::projection::project_display_infos(&list, &[]);
        self.navigator.provider_settings.set_providers(list);
        self.navigator
            .provider_settings
            .set_provider_display_infos(display);
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
                    // Refresh the list so the configured indicator
                    // repaints.
                    if let Ok(list) = backend.list_provider_credentials().await {
                        let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_for_save, "set_provider_credentials failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054: spawn a connection-test round-trip and route the
    /// `TestConnectionResult` back through `Action::ProviderTestComplete`.
    pub(crate) fn handle_test_provider_connection(&mut self, provider_id: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.test_provider_connection(provider_id.clone()).await {
                Ok(result) => {
                    let _ = action_tx.send(Action::ProviderTestComplete {
                        provider_id,
                        result,
                    });
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_id, "test_provider_connection failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054: fold a `TestConnectionResult` into the view's status
    /// area. Success renders as "✓ ok (Xms)" and failure as
    /// "✗ <error>".
    pub(crate) fn handle_provider_test_complete(
        &mut self,
        provider_id: String,
        result: codelet_rpc_types::TestConnectionResult,
    ) {
        let status = if result.success {
            format!("✓ {} ok ({}ms)", provider_id, result.latency_ms)
        } else {
            format!(
                "✗ {}",
                result.error.unwrap_or_else(|| "unknown error".to_string())
            )
        };
        self.navigator.provider_settings.set_status(status);
    }

    /// RPC-054: spawn a refresh-models round-trip and follow up with a
    /// fresh list refresh so the view repaints with the new model count.
    pub(crate) fn handle_refresh_provider_models(&mut self, provider_id: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.refresh_models_cache(provider_id.clone()).await {
                Ok(models) => {
                    let _ = action_tx.send(Action::ProviderModelsRefreshed {
                        provider_id: provider_id.clone(),
                        model_count: models.len() as u32,
                    });
                    if let Ok(list) = backend.list_provider_credentials().await {
                        let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, provider = %provider_id, "refresh_models_cache failed");
                    let _ = action_tx.send(Action::ProviderSettingsStatus(format!("✗ {e}")));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-054: fold a `ProviderModelsRefreshed` into the view's status
    /// area.
    pub(crate) fn handle_provider_models_refreshed(
        &mut self,
        provider_id: String,
        model_count: u32,
    ) {
        self.navigator.provider_settings.set_status(format!(
            "✓ {provider_id} models refreshed ({model_count} models)"
        ));
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
                    if let Ok(list) = backend.list_provider_credentials().await {
                        let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
                    }
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
            Action::ProviderCredentialsLoaded(list) => {
                self.handle_provider_credentials_loaded(list.clone());
            }
            Action::SaveProviderCredentials {
                provider_id,
                api_key,
            } => {
                self.handle_save_provider_credentials(provider_id.clone(), api_key.clone());
            }
            Action::TestProviderConnection(id) => {
                self.handle_test_provider_connection(id.clone());
            }
            Action::ProviderTestComplete {
                provider_id,
                result,
            } => {
                self.handle_provider_test_complete(provider_id.clone(), result.clone());
            }
            Action::RefreshProviderModels(id) => {
                self.handle_refresh_provider_models(id.clone());
            }
            Action::ProviderModelsRefreshed {
                provider_id,
                model_count,
            } => {
                self.handle_provider_models_refreshed(provider_id.clone(), *model_count);
            }
            Action::DeleteProviderCredentials(id) => {
                self.handle_delete_provider_credentials(id.clone());
            }
            Action::ConfirmDeleteProviderCredentials(id) => {
                // RPC-054 (revision): ConfirmDialog Primary acceptance
                // routes through the same backend round-trip as the
                // legacy raw `DeleteProviderCredentials` arm. The
                // view-layer is now responsible for opening the
                // dialog BEFORE this action fires.
                self.handle_delete_provider_credentials(id.clone());
            }
            Action::ProviderSettingsStatus(s) => {
                self.handle_provider_settings_status(s.clone());
            }
            _ => return false,
        }
        true
    }
}
