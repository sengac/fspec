//! PROV-114 — view-side key handling for the github-copilot OAuth device
//! preamble modes (deployment-type-select + enterprise-host entry).
//!
//! Feature: spec/features/provider-settings-oauth-copilot-device.feature
//!
//! Split out of `oauth_login.rs` to keep both under the 300-LoC ceiling and to
//! isolate the copilot-only flow. Owns:
//!   * `start_deployment_select` — Enter on the github-copilot `OAuthLogin` row
//!     routes here FIRST (by `provider_id == "github-copilot"`, before method),
//!     entering `OAuthDeploymentTypeSelect` with "GitHub.com" (index 0)
//!     selected.
//!   * `handle_copilot_preamble_key` — the deployment-type-select /
//!     enterprise-url-entry keyboard contracts: ↑/↓ select; Enter on index 0
//!     begins device polling (no host), Enter on index 1 enters enterprise
//!     entry; enterprise entry appends printable chars / pops on
//!     Backspace+Delete (clearing the validation error), Enter-empty sets the
//!     validation error, Enter-nonempty normalizes the host and begins device
//!     polling; Esc cancels both back to the list.
//!
//! Host normalization uses `codelet_providers::copilot::
//! normalize_enterprise_domain` (sync, pure) so the emitted
//! `OAuthCopilotDeviceStart` carries the bare host. The device-waiting /
//! success / error screens themselves are the shared PROV-113 modes.

use codelet_providers::copilot::normalize_enterprise_domain;
use crossterm::event::{KeyCode, KeyEvent};

use crate::components::Action;

use super::oauth_login::cancel_to_list;
use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

const GITHUB_COPILOT: &str = "github-copilot";
const EMPTY_URL_ERROR: &str = "URL or domain is required";

/// Enter on the github-copilot `OAuthLogin` row: enter the deployment-type
/// preamble with "GitHub.com" (index 0) selected. No backend call yet.
pub(super) fn start_deployment_select(
    view: &mut ProviderSettingsView,
    provider_id: String,
) -> ProviderSettingsEvent {
    view.oauth_last_provider = Some(provider_id.clone());
    view.status.clear();
    view.mode = ProviderSettingsMode::OAuthDeploymentTypeSelect {
        provider_id,
        selected_index: 0,
    };
    ProviderSettingsEvent::Consumed
}

/// Route a key through one of the two copilot preamble modes.
pub(super) fn handle_copilot_preamble_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    mode: ProviderSettingsMode,
) -> ProviderSettingsEvent {
    match mode {
        ProviderSettingsMode::OAuthDeploymentTypeSelect {
            provider_id,
            selected_index,
        } => handle_deployment_select_key(view, key, provider_id, selected_index),
        ProviderSettingsMode::OAuthEnterpriseUrlEntry {
            provider_id,
            url_input,
            validation_error,
        } => handle_enterprise_entry_key(view, key, provider_id, url_input, validation_error),
        // Unreachable: only the two copilot preamble modes route here.
        _ => ProviderSettingsEvent::Consumed,
    }
}

/// Deployment-type-select: ↑ → index 0, ↓ → index 1, Enter routes by index,
/// Esc cancels. Any other key is consumed (stays open).
fn handle_deployment_select_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    selected_index: usize,
) -> ProviderSettingsEvent {
    match key.code {
        KeyCode::Esc => cancel_to_list(view),
        KeyCode::Up => {
            view.mode = ProviderSettingsMode::OAuthDeploymentTypeSelect {
                provider_id,
                selected_index: 0,
            };
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Down => {
            view.mode = ProviderSettingsMode::OAuthDeploymentTypeSelect {
                provider_id,
                selected_index: 1,
            };
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Enter => {
            if selected_index == 0 {
                begin_device_polling(view, provider_id, None)
            } else {
                view.mode = ProviderSettingsMode::OAuthEnterpriseUrlEntry {
                    provider_id,
                    url_input: String::new(),
                    validation_error: None,
                };
                ProviderSettingsEvent::Consumed
            }
        }
        _ => {
            view.mode = ProviderSettingsMode::OAuthDeploymentTypeSelect {
                provider_id,
                selected_index,
            };
            ProviderSettingsEvent::Consumed
        }
    }
}

/// Enterprise-url-entry: printable chars append (clearing the error),
/// Backspace/Delete pop (clearing the error), Enter-empty sets the validation
/// error, Enter-nonempty normalizes the host and begins device polling, Esc
/// cancels.
fn handle_enterprise_entry_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    mut url_input: String,
    mut validation_error: Option<String>,
) -> ProviderSettingsEvent {
    match key.code {
        KeyCode::Esc => return cancel_to_list(view),
        KeyCode::Enter => {
            if url_input.trim().is_empty() {
                validation_error = Some(EMPTY_URL_ERROR.to_string());
            } else {
                let host = normalize_enterprise_domain(&url_input);
                return begin_device_polling(view, provider_id, Some(host));
            }
        }
        // Only printable ASCII (32..=126) appends and clears the error;
        // control / non-ASCII chars are dropped (mirrors the PROV-113
        // headless-code-entry filter and copilotOauthModeHandler.ts).
        KeyCode::Char(ch) if (' '..='~').contains(&ch) => {
            url_input.push(ch);
            validation_error = None;
        }
        KeyCode::Backspace | KeyCode::Delete => {
            url_input.pop();
            validation_error = None;
        }
        _ => {}
    }
    view.mode = ProviderSettingsMode::OAuthEnterpriseUrlEntry {
        provider_id,
        url_input,
        validation_error,
    };
    ProviderSettingsEvent::Consumed
}

/// Enter the shared device-waiting screen (empty code/URL until the
/// device-start result arrives) and emit the copilot device-start action,
/// carrying `enterprise_host` (None for GitHub.com) and the current generation.
fn begin_device_polling(
    view: &mut ProviderSettingsView,
    provider_id: String,
    enterprise_host: Option<String>,
) -> ProviderSettingsEvent {
    let generation = view.oauth_generation;
    view.oauth_last_provider = Some(provider_id.clone());
    view.status.clear();
    view.mode = ProviderSettingsMode::OAuthDeviceWaiting {
        provider_id,
        user_code: String::new(),
        verification_url: String::new(),
    };
    ProviderSettingsEvent::Emit(Action::OAuthCopilotDeviceStart {
        enterprise_host,
        generation,
    })
}

/// PROV-114: retry the github-copilot device login from the error screen
/// (Enter). Always restarts the GitHub.com branch (no enterprise host); the
/// user can re-pick enterprise from the list.
pub(super) fn retry_copilot_login(
    view: &mut ProviderSettingsView,
    provider_id: String,
) -> ProviderSettingsEvent {
    begin_device_polling(view, provider_id, None)
}

/// Whether `provider_id` is the github-copilot provider (drives the
/// login-row + error-retry routing).
pub(super) fn is_copilot(provider_id: &str) -> bool {
    provider_id == GITHUB_COPILOT
}
