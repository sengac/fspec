//! BUG-575: the numbered Provider Settings refresh.
//!
//! Feature: spec/features/provider-settings-loads-and-writes-never-corrupt-the-view.feature
//!
//! Opening /provider and every write that changes what it shows request a
//! refresh here. The App numbers it when it is requested, reads the
//! credentials and then the settings in one task, and sends both in one
//! `ProviderSettingsRefreshed`; the view shows it only when it is newer than
//! the one shown, so refreshes that finish in reverse order leave the newest
//! state on screen.

use crate::components::Action;
use crate::views::provider_settings::refresh::{
    NavigateRequest, NavigateTarget, ProviderSettingsRefresh,
};

use super::state::App;

impl App {
    /// Request a refresh; `navigate` returns the cursor once it is shown.
    pub(crate) fn spawn_provider_settings_refresh(&mut self, navigate: Option<NavigateRequest>) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        self.provider_settings_refreshes += 1;
        let generation = self.provider_settings_refreshes;
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            let credentials = backend
                .list_provider_credentials()
                .await
                .map_err(|error| format!("{error:#}"));
            let configs = backend
                .list_provider_settings()
                .await
                .map_err(|error| error.to_string());
            if let Err(error) = &credentials {
                tracing::warn!(%error, "list_provider_credentials failed");
            }
            if let Err(error) = &configs {
                tracing::warn!(%error, "list_provider_settings failed");
            }
            let _ = action_tx.send(Action::ProviderSettingsRefreshed(Box::new(
                ProviderSettingsRefresh {
                    generation,
                    credentials,
                    configs,
                    navigate,
                },
            )));
        }));
    }

    /// The first refresh of a freshly opened view, which ignores every
    /// refresh requested before it opened.
    pub(crate) fn refresh_new_provider_settings_view(&mut self) {
        self.navigator
            .provider_settings
            .refresh_floor(self.provider_settings_refreshes);
        self.spawn_provider_settings_refresh(None);
    }

    /// A write's cursor target, stamped with the current navigation epoch.
    pub(crate) fn navigate_request(&self, target: NavigateTarget) -> NavigateRequest {
        NavigateRequest {
            target,
            nav_epoch: self.navigator.provider_settings.nav_epoch(),
        }
    }
}
