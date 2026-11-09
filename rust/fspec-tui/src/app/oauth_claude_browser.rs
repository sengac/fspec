use codelet_providers::claude_oauth_server::ClaudeBrowserAuthorization;

use crate::components::Action;

use super::{login_terminal_action, App};

impl App {
    pub(super) fn handle_client_claude_browser_login_start(
        &mut self,
        provider_id: String,
        generation: u64,
    ) {
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let result = async {
                let authorization = ClaudeBrowserAuthorization::bind().await?;
                action_tx.send(Action::OAuthBrowserReady {
                    provider_id: provider_id.clone(),
                    authorize_url: authorization.browser_url().to_owned(),
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
                tracing::error!(%error, "Cannot deliver Anthropic browser authorization result");
            }
        });
        self.navigator.provider_settings.track_oauth_task(&handle);
        self.pending_tasks.push(handle);
    }
}
