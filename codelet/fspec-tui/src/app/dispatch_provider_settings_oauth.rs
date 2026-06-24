//! PROV-112 / PROV-113 — OAuth write surface for the ProviderSettingsView.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Split out of `dispatch_provider_settings.rs` to keep that file under the
//! 300-LoC ceiling. PROV-112 owns the disconnect/logout clear; PROV-113 adds
//! the browser / headless / device LOGIN flows. Every async call is spawned as
//! a tokio task (mirroring the PROV-109 profile write loop); the task sends a
//! follow-up `Action` carrying the originating `generation` so a result whose
//! generation no longer matches the view's (the user pressed Esc) is dropped.

use tokio::task::JoinHandle;

use crate::components::Action;
use crate::views::provider_settings::nav_item::OAuthMethod;

use super::state::App;

impl App {
    /// PROV-112/113: route an OAuth action. Called from
    /// `try_dispatch_provider_settings`. Returns `true` if handled.
    pub(crate) fn try_dispatch_oauth(&mut self, action: &Action) -> bool {
        match action {
            Action::OAuthDisconnect { provider_id } => {
                self.handle_oauth_disconnect(provider_id.clone());
            }
            Action::OAuthLoginStart {
                provider_id,
                method,
                generation,
            } => self.handle_oauth_login_start(provider_id.clone(), *method, *generation),
            Action::OAuthHeadlessReady {
                provider_id,
                authorize_url,
                pkce_verifier,
                generation,
            } => self.handle_oauth_headless_ready(
                provider_id.clone(),
                authorize_url.clone(),
                pkce_verifier.clone(),
                *generation,
            ),
            Action::OAuthDeviceReady {
                provider_id,
                user_code,
                verification_url,
                device_auth_id,
                interval,
                generation,
            } => self.handle_oauth_device_ready(
                provider_id.clone(),
                user_code.clone(),
                verification_url.clone(),
                device_auth_id.clone(),
                *interval,
                *generation,
            ),
            Action::OAuthLoginHeadlessSubmit {
                provider_id,
                code,
                pkce_verifier,
                generation,
            } => self.handle_oauth_headless_submit(
                provider_id.clone(),
                code.clone(),
                pkce_verifier.clone(),
                *generation,
            ),
            Action::OAuthLoginSucceeded {
                provider_id,
                generation,
            } => self.handle_oauth_login_succeeded(provider_id.clone(), *generation),
            Action::OAuthLoginFailed {
                provider_id,
                error,
                generation,
            } => self.handle_oauth_login_failed(provider_id.clone(), error.clone(), *generation),
            Action::OAuthOpenUrl { url } => {
                tracing::info!(url = %url, "oauth open-url requested");
            }
            Action::OAuthCopyUrl { url } => {
                tracing::info!(url = %url, "oauth copy-url requested");
            }
            _ => return false,
        }
        true
    }

    /// PROV-112: clear a provider's OAuth tokens then refresh the nav.
    pub(crate) fn handle_oauth_disconnect(&mut self, provider_id: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        self.navigator
            .provider_settings
            .set_navigate_target(provider_id.clone());
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if let Err(e) = backend.oauth_clear_tokens(provider_id.clone()).await {
                tracing::warn!(error = %e, provider = %provider_id, "oauth_clear_tokens failed");
            }
            if let Ok(list) = backend.list_provider_credentials().await {
                let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-113: start a login. Browser runs fire-and-forget; headless routes
    /// to a `start` whose result Action picks the code-entry/device mode.
    fn handle_oauth_login_start(&mut self, provider_id: String, method: OAuthMethod, gen: u64) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match method {
                OAuthMethod::Browser => {
                    let result = backend.oauth_browser_login(provider_id.clone()).await;
                    let _ = action_tx.send(login_terminal_action(provider_id, gen, result));
                }
                OAuthMethod::Headless if provider_id == "anthropic" => {
                    match backend.oauth_headless_start(provider_id.clone()).await {
                        Ok(start) => {
                            let _ = action_tx.send(Action::OAuthHeadlessReady {
                                provider_id,
                                authorize_url: start.authorize_url,
                                pkce_verifier: start.pkce_verifier,
                                generation: gen,
                            });
                        }
                        Err(e) => {
                            let _ = action_tx.send(fail_action(provider_id, gen, &e));
                        }
                    }
                }
                OAuthMethod::Headless => {
                    match backend.oauth_device_start(provider_id.clone()).await {
                        Ok(start) => {
                            let _ = action_tx.send(Action::OAuthDeviceReady {
                                provider_id,
                                user_code: start.user_code,
                                verification_url: start.verification_url,
                                device_auth_id: start.device_auth_id,
                                interval: start.interval,
                                generation: gen,
                            });
                        }
                        Err(e) => {
                            let _ = action_tx.send(fail_action(provider_id, gen, &e));
                        }
                    }
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-113: anthropic headless start resolved — enter code-entry (unless
    /// the flow was cancelled, in which case the result is dropped).
    fn handle_oauth_headless_ready(
        &mut self,
        provider_id: String,
        authorize_url: String,
        pkce_verifier: String,
        gen: u64,
    ) {
        let view = &mut self.navigator.provider_settings;
        if view.oauth_generation() != gen {
            return;
        }
        view.mode = crate::views::ProviderSettingsMode::OAuthHeadlessCodeEntry {
            provider_id,
            authorize_url,
            pkce_verifier,
            code_input: String::new(),
        };
    }

    /// PROV-113: codex device start resolved — show device-waiting and begin
    /// polling (dropped when cancelled).
    fn handle_oauth_device_ready(
        &mut self,
        provider_id: String,
        user_code: String,
        verification_url: String,
        device_auth_id: String,
        interval: u64,
        gen: u64,
    ) {
        {
            let view = &mut self.navigator.provider_settings;
            if view.oauth_generation() != gen {
                return;
            }
            view.mode = crate::views::ProviderSettingsMode::OAuthDeviceWaiting {
                provider_id: provider_id.clone(),
                user_code,
                verification_url,
            };
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let result = backend
                .oauth_device_poll(provider_id.clone(), device_auth_id, interval)
                .await;
            let _ = action_tx.send(login_terminal_action(provider_id, gen, result));
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-113: submit the pasted headless code and await completion.
    fn handle_oauth_headless_submit(
        &mut self,
        provider_id: String,
        code: String,
        pkce_verifier: String,
        gen: u64,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let result = backend
                .oauth_headless_complete(provider_id.clone(), code, pkce_verifier)
                .await;
            let _ = action_tx.send(login_terminal_action(provider_id, gen, result));
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-113: a login succeeded — show the success screen and refresh the
    /// nav so the Logout row appears (dropped when the flow was cancelled).
    fn handle_oauth_login_succeeded(&mut self, provider_id: String, gen: u64) {
        {
            let view = &mut self.navigator.provider_settings;
            if view.oauth_generation() != gen {
                return;
            }
            view.mode = crate::views::ProviderSettingsMode::OAuthSuccess {
                provider_id: provider_id.clone(),
            };
            view.set_navigate_target(provider_id);
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            if let Ok(list) = backend.list_provider_credentials().await {
                let _ = action_tx.send(Action::ProviderCredentialsLoaded(list));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// PROV-113: a login failed — show the error screen (dropped when the flow
    /// was cancelled).
    fn handle_oauth_login_failed(&mut self, provider_id: String, error: String, gen: u64) {
        let view = &mut self.navigator.provider_settings;
        if view.oauth_generation() != gen {
            return;
        }
        view.mode = crate::views::ProviderSettingsMode::OAuthError { provider_id, error };
    }
}

/// Map a terminal `Result<(), anyhow::Error>` to the matching login Action.
fn login_terminal_action(
    provider_id: String,
    generation: u64,
    result: anyhow::Result<()>,
) -> Action {
    match result {
        Ok(()) => Action::OAuthLoginSucceeded {
            provider_id,
            generation,
        },
        Err(e) => fail_action(provider_id, generation, &e),
    }
}

fn fail_action(provider_id: String, generation: u64, error: &anyhow::Error) -> Action {
    Action::OAuthLoginFailed {
        provider_id,
        error: error.to_string(),
        generation,
    }
}
