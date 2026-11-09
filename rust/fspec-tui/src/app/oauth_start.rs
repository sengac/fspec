use crate::components::Action;
use crate::views::provider_settings::nav_item::OAuthMethod;

use super::{fail_action, App};

impl App {
    pub(super) fn handle_oauth_login_start(
        &mut self,
        provider_id: String,
        method: OAuthMethod,
        generation: u64,
    ) {
        if self.navigator.provider_settings.oauth_generation() != generation {
            return;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        match method {
            // BUG-600: browser sign-in runs on the client on every transport,
            // and only Anthropic and Codex offer it.
            OAuthMethod::Browser => match provider_id.as_str() {
                "anthropic" => {
                    self.handle_client_claude_browser_login_start(provider_id, generation)
                }
                "codex" => self.handle_client_browser_login_start(provider_id, generation),
                _ => {
                    let error = anyhow::anyhow!("{provider_id} has no browser sign-in");
                    let _ = self
                        .action_tx
                        .send(fail_action(provider_id, generation, &error));
                }
            },
            OAuthMethod::Headless => self.handle_headless_login_start(provider_id, generation),
        }
    }

    fn handle_headless_login_start(&mut self, provider_id: String, generation: u64) {
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if provider_id == "anthropic" {
                match backend.oauth_headless_start(provider_id.clone()).await {
                    Ok(start) => {
                        let _ = action_tx.send(Action::OAuthHeadlessReady {
                            provider_id,
                            authorize_url: start.authorize_url,
                            pkce_verifier: start.pkce_verifier,
                            generation,
                        });
                    }
                    Err(error) => {
                        let _ = action_tx.send(fail_action(provider_id, generation, &error));
                    }
                }
            } else {
                match backend.oauth_device_start(provider_id.clone()).await {
                    Ok(start) => {
                        let _ = action_tx.send(Action::OAuthDeviceReady {
                            provider_id,
                            user_code: start.user_code,
                            verification_url: start.verification_url,
                            device_auth_id: start.device_auth_id,
                            interval: start.interval,
                            generation,
                        });
                    }
                    Err(error) => {
                        let _ = action_tx.send(fail_action(provider_id, generation, &error));
                    }
                }
            }
        });
        self.navigator.provider_settings.track_oauth_task(&handle);
        self.pending_tasks.push(handle);
    }
}
