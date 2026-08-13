//! PROV-114 — github-copilot device-start dispatch for the
//! ProviderSettingsView.
//!
//! Feature: spec/features/provider-settings-oauth-copilot-device.feature
//!
//! Split out of `dispatch_provider_settings_oauth.rs` (which is at the 300-LoC
//! ceiling) so the copilot device-start lives in its own sibling. The handler
//! spawns `backend.oauth_copilot_device_start(enterprise_host)` via the
//! codelet-providers-direct embedded transport; on success it routes back
//! through the SHARED PROV-113 `Action::OAuthDeviceReady` →
//! `oauth_device_poll` machinery (device-waiting → success/error); a start
//! failure routes to the shared `Action::OAuthLoginFailed` (oauth-error). Both
//! carry the originating `generation` so a result whose generation no longer
//! matches the view's (the user pressed Esc) is dropped. The provider is always
//! `github-copilot`; errors are stringified UI-safe (no RPC/method name leaks).

use tokio::task::JoinHandle;

use crate::components::Action;

use super::state::App;

const GITHUB_COPILOT: &str = "github-copilot";

impl App {
    /// PROV-114: route the copilot device-start action. Called from
    /// `try_dispatch_provider_settings`. Returns `true` if handled.
    pub(crate) fn try_dispatch_copilot_oauth(&mut self, action: &Action) -> bool {
        if let Action::OAuthCopilotDeviceStart {
            enterprise_host,
            generation,
        } = action
        {
            self.handle_oauth_copilot_device_start(enterprise_host.clone(), *generation);
            return true;
        }
        false
    }

    /// PROV-114: begin the copilot device flow. `enterprise_host` is `None` for
    /// GitHub.com or `Some(normalized_host)` for GitHub Enterprise. On success
    /// emit `OAuthDeviceReady` (which shows device-waiting + starts the poll);
    /// on failure emit `OAuthLoginFailed` (oauth-error).
    fn handle_oauth_copilot_device_start(&mut self, enterprise_host: Option<String>, gen: u64) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let action = match backend.oauth_copilot_device_start(enterprise_host).await {
                Ok(start) => Action::OAuthDeviceReady {
                    provider_id: GITHUB_COPILOT.to_string(),
                    user_code: start.user_code,
                    verification_url: start.verification_url,
                    device_auth_id: start.device_auth_id,
                    interval: start.interval,
                    generation: gen,
                },
                Err(e) => Action::OAuthLoginFailed {
                    provider_id: GITHUB_COPILOT.to_string(),
                    error: e.to_string(),
                    generation: gen,
                },
            };
            let _ = action_tx.send(action);
        });
        self.pending_tasks.push(handle);
    }
}
