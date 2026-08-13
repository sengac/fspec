//! PROV-113 — view-side key handling for the OAuth login modes.
//!
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Split out of `list_actions.rs`/`mod.rs` to keep both under the 300-LoC
//! ceiling. Owns:
//!   * `start_oauth_login` — Enter on an `OAuthLogin` row routes here, keyed by
//!     (provider, method): a Browser row enters `OAuthBrowserWaiting`
//!     immediately; a Headless row emits the start action and lets the
//!     dispatch result pick the code-entry (anthropic) / device-waiting (codex)
//!     mode. Both emit `Action::OAuthLoginStart { provider, method, generation }`.
//!   * `handle_oauth_login_key` — the waiting / device-waiting / code-entry /
//!     success / error mode keyboard contracts, including the generation bump
//!     on Esc-cancel and the `c`-copies-while-empty / `o`-opens-while-empty
//!     code-entry rules.

use crossterm::event::{KeyCode, KeyEvent};

use crate::components::Action;

use super::nav_item::OAuthMethod;
use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// Enter on an `OAuthLogin` row. Browser rows enter the waiting screen at
/// once; headless rows defer the mode to the dispatch start-result. Both emit
/// the start action carrying the current generation.
pub(super) fn start_oauth_login(
    view: &mut ProviderSettingsView,
    provider_id: String,
    method: OAuthMethod,
) -> ProviderSettingsEvent {
    let generation = view.oauth_generation;
    view.oauth_last_provider = Some(provider_id.clone());
    view.oauth_last_method = Some(method);
    view.status.clear();
    if matches!(method, OAuthMethod::Browser) {
        view.mode = ProviderSettingsMode::OAuthBrowserWaiting {
            provider_id: provider_id.clone(),
        };
    }
    ProviderSettingsEvent::Emit(Action::OAuthLoginStart {
        provider_id,
        method,
        generation,
    })
}

/// Route a key through one of the five OAuth login modes.
pub(super) fn handle_oauth_login_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    mode: ProviderSettingsMode,
) -> ProviderSettingsEvent {
    match mode {
        ProviderSettingsMode::OAuthBrowserWaiting { .. }
        | ProviderSettingsMode::OAuthDeviceWaiting { .. } => match key.code {
            KeyCode::Esc => cancel_to_list(view),
            _ => ProviderSettingsEvent::Consumed,
        },
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            provider_id,
            authorize_url,
            pkce_verifier,
            code_input,
        } => handle_code_entry_key(
            view,
            key,
            provider_id,
            authorize_url,
            pkce_verifier,
            code_input,
        ),
        ProviderSettingsMode::OAuthSuccess { .. } => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                view.mode = ProviderSettingsMode::List;
                view.status.clear();
                ProviderSettingsEvent::Consumed
            }
            _ => ProviderSettingsEvent::Consumed,
        },
        ProviderSettingsMode::OAuthError { provider_id, .. } => match key.code {
            KeyCode::Enter => retry_last_login(view, provider_id),
            KeyCode::Esc => cancel_to_list(view),
            _ => ProviderSettingsEvent::Consumed,
        },
        // Unreachable: only the five OAuth login modes are routed here.
        _ => ProviderSettingsEvent::Consumed,
    }
}

/// Esc during a login: bump the generation (so a late result is dropped) and
/// return to the list. No backend cancel is needed — the spawned task's result
/// is simply ignored on arrival.
pub(super) fn cancel_to_list(view: &mut ProviderSettingsView) -> ProviderSettingsEvent {
    view.oauth_generation = view.oauth_generation.wrapping_add(1);
    view.mode = ProviderSettingsMode::List;
    view.status.clear();
    ProviderSettingsEvent::Consumed
}

/// Retry the last login from the error screen. The github-copilot device flow
/// retries via its own preamble helper (no `OAuthMethod`); browser re-enters
/// the waiting screen immediately; headless defers to the dispatch
/// start-result.
fn retry_last_login(view: &mut ProviderSettingsView, provider_id: String) -> ProviderSettingsEvent {
    if super::oauth_copilot::is_copilot(&provider_id) {
        return super::oauth_copilot::retry_copilot_login(view, provider_id);
    }
    let method = view.oauth_last_method.unwrap_or(OAuthMethod::Browser);
    start_oauth_login(view, provider_id, method)
}

/// The code-entry keyboard contract (anthropic headless):
///   * Esc            → cancel (generation bump, back to list)
///   * Enter          → submit ONLY when `code_input` is non-empty
///   * `c` (empty)    → copy the authorize URL to the clipboard
///   * `o` (empty)    → open the authorize URL in the browser
///   * any other char → append (so `c`/`o` are literal once input is non-empty)
///   * Backspace      → delete the last char
fn handle_code_entry_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    authorize_url: String,
    pkce_verifier: String,
    mut code_input: String,
) -> ProviderSettingsEvent {
    let event = match key.code {
        KeyCode::Esc => return cancel_to_list(view),
        KeyCode::Enter => {
            if code_input.is_empty() {
                ProviderSettingsEvent::Consumed
            } else {
                ProviderSettingsEvent::Emit(Action::OAuthLoginHeadlessSubmit {
                    provider_id: provider_id.clone(),
                    code: code_input.clone(),
                    pkce_verifier: pkce_verifier.clone(),
                    generation: view.oauth_generation,
                })
            }
        }
        KeyCode::Char('c') if code_input.is_empty() => {
            ProviderSettingsEvent::Emit(Action::OAuthCopyUrl {
                url: authorize_url.clone(),
            })
        }
        KeyCode::Char('o') if code_input.is_empty() => {
            ProviderSettingsEvent::Emit(Action::OAuthOpenUrl {
                url: authorize_url.clone(),
            })
        }
        KeyCode::Char(ch) => {
            code_input.push(ch);
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Backspace => {
            code_input.pop();
            ProviderSettingsEvent::Consumed
        }
        _ => ProviderSettingsEvent::Consumed,
    };
    // Rewrite the mode with the (possibly) updated input — Esc/Enter-submit
    // already returned above, so we always stay in code-entry here.
    view.mode = ProviderSettingsMode::OAuthHeadlessCodeEntry {
        provider_id,
        authorize_url,
        pkce_verifier,
        code_input,
    };
    event
}
