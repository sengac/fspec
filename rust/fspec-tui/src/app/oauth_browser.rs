use codelet_providers::codex::codex_browser_authorization::CodexBrowserAuthorization;

use crate::components::Action;
use crate::views::ProviderSettingsMode;

use super::{login_terminal_action, App};

impl App {
    pub(super) fn handle_client_browser_login_start(
        &mut self,
        provider_id: String,
        generation: u64,
    ) {
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let result = async {
                let start = backend.oauth_headless_start(provider_id.clone()).await?;
                let authorization = CodexBrowserAuthorization::bind_for_login(
                    start.authorize_url,
                    start.pkce_verifier,
                )
                .await?;
                action_tx.send(Action::OAuthBrowserReady {
                    provider_id: provider_id.clone(),
                    authorize_url: authorization.authorize_url().to_owned(),
                    generation,
                })?;
                let (code, verifier) = authorization.complete().await?;
                backend
                    .oauth_headless_complete(provider_id.clone(), code, verifier)
                    .await
            }
            .await;
            if let Err(error) =
                action_tx.send(login_terminal_action(provider_id, generation, result))
            {
                tracing::error!(%error, "Cannot deliver browser authorization result");
            }
        });
        self.navigator.provider_settings.track_oauth_task(&handle);
        self.pending_tasks.push(handle);
    }

    pub(super) fn handle_oauth_browser_ready(
        &mut self,
        provider_id: String,
        authorize_url: String,
        generation: u64,
    ) {
        let view = &mut self.navigator.provider_settings;
        if view.oauth_generation() != generation {
            return;
        }
        view.mode = ProviderSettingsMode::OAuthBrowserWaiting { provider_id };
        view.oauth_browser_url = Some(authorize_url.clone());
        self.open_url(authorize_url, "Press c to copy the URL.");
    }
}
