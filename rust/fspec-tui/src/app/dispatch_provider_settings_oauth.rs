//! PROV-112 / PROV-113 — OAuth write surface for the ProviderSettingsView.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Login tasks are owned by the settings view for cancellation on Esc/drop.
//! Generation fences also reject already-queued results after cancellation.

use tokio::task::JoinHandle;

use crate::components::Action;

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
            Action::OAuthBrowserReady {
                provider_id,
                authorize_url,
                generation,
            } => {
                self.handle_oauth_browser_ready(
                    provider_id.clone(),
                    authorize_url.clone(),
                    *generation,
                );
            }
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
            Action::OAuthOpenUrl { url } => self.handle_oauth_open_url(url.clone()),
            _ => return false,
        }
        true
    }

    /// PROV-112: clear a provider's OAuth tokens then refresh the nav.
    pub(crate) fn handle_oauth_disconnect(&mut self, provider_id: String) {
        disconnect::start(self, provider_id);
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
            code_focused: false,
        };
    }

    /// TUI-126: open a sign-in URL. When no browser opens, the status says
    /// what to do instead: copy the URL (code entry) or visit the URL shown.
    fn handle_oauth_open_url(&mut self, url: String) {
        let instead = match self.navigator.provider_settings.mode {
            crate::views::ProviderSettingsMode::OAuthHeadlessCodeEntry { .. } => {
                "Press c to copy the URL."
            }
            _ => "Visit the URL shown on screen.",
        };
        self.open_url(url, instead);
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
        self.navigator.provider_settings.track_oauth_task(&handle);
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
        self.navigator.provider_settings.track_oauth_task(&handle);
        self.pending_tasks.push(handle);
    }

    /// PROV-113: a login succeeded — show the success screen and refresh the
    /// nav so the Logout row appears (dropped when the flow was cancelled).
    /// BUG-664: the refresh is the numbered one every write uses, and it
    /// returns the cursor to the provider unless the user moves it first.
    fn handle_oauth_login_succeeded(&mut self, provider_id: String, gen: u64) {
        let view = &mut self.navigator.provider_settings;
        if view.oauth_generation() != gen {
            return;
        }
        view.mode = crate::views::ProviderSettingsMode::OAuthSuccess {
            provider_id: provider_id.clone(),
        };
        let navigate = self.navigate_request(
            crate::views::provider_settings::refresh::NavigateTarget::Provider(provider_id),
        );
        self.spawn_provider_settings_refresh(Some(navigate));
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

#[path = "oauth_disconnect.rs"]
mod disconnect;

#[path = "oauth_start.rs"]
mod start;

#[path = "oauth_browser.rs"]
mod browser;

#[path = "oauth_claude_browser.rs"]
mod claude_browser;
