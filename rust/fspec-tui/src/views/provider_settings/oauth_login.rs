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
//!     on Esc-cancel. TUI-126: code entry has a URL part and a Code field;
//!     on the URL `c` copies and `o` opens it, in the Code field every
//!     printable character is code text; device login copies its user code
//!     and opens its verification URL; the error screen copies its text.

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
    view.oauth_browser_url = None;
    if matches!(method, OAuthMethod::Browser) {
        view.mode = ProviderSettingsMode::OAuthBrowserWaiting {
            provider_id: provider_id.clone(),
        };
    }
    ProviderSettingsEvent::Emit(Box::new(Action::OAuthLoginStart {
        provider_id,
        method,
        generation,
    }))
}

/// Route a key through one of the five OAuth login modes.
pub(super) fn handle_oauth_login_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    mode: ProviderSettingsMode,
) -> ProviderSettingsEvent {
    match mode {
        ProviderSettingsMode::OAuthBrowserWaiting { .. } => match key.code {
            KeyCode::Esc => cancel_to_list(view),
            KeyCode::Char('c') => match &view.oauth_browser_url {
                Some(url) => copy(url.clone()),
                None => ProviderSettingsEvent::Consumed,
            },
            KeyCode::Char('o') => match &view.oauth_browser_url {
                Some(url) => open(url.clone()),
                None => ProviderSettingsEvent::Consumed,
            },
            _ => ProviderSettingsEvent::Consumed,
        },
        // TUI-126: c copies the user code, o opens the verification URL; both
        // do nothing while the device code is still being requested.
        ProviderSettingsMode::OAuthDeviceWaiting {
            user_code,
            verification_url,
            ..
        } => match key.code {
            KeyCode::Esc => cancel_to_list(view),
            KeyCode::Char('c') if !user_code.is_empty() => copy(user_code),
            KeyCode::Char('o') if !verification_url.is_empty() => open(verification_url),
            _ => ProviderSettingsEvent::Consumed,
        },
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            provider_id,
            authorize_url,
            pkce_verifier,
            code_input,
            code_focused,
        } => handle_code_entry_key(
            view,
            key,
            CodeEntry {
                provider_id,
                authorize_url,
                pkce_verifier,
                code_input,
                code_focused,
            },
        ),
        ProviderSettingsMode::OAuthSuccess { .. } => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                view.mode = ProviderSettingsMode::List;
                view.status.clear();
                ProviderSettingsEvent::Consumed
            }
            _ => ProviderSettingsEvent::Consumed,
        },
        ProviderSettingsMode::OAuthError { provider_id, error } => match key.code {
            KeyCode::Enter => retry_last_login(view, provider_id),
            KeyCode::Esc => cancel_to_list(view),
            KeyCode::Char('c') => copy(error),
            _ => ProviderSettingsEvent::Consumed,
        },
        // Unreachable: only the five OAuth login modes are routed here.
        _ => ProviderSettingsEvent::Consumed,
    }
}

pub(super) fn cancel_to_list(view: &mut ProviderSettingsView) -> ProviderSettingsEvent {
    view.cancel_oauth_tasks();
    view.oauth_browser_url = None;
    view.oauth_generation = view.oauth_generation.wrapping_add(1);
    view.mode = ProviderSettingsMode::List;
    view.status.clear();
    ProviderSettingsEvent::Consumed
}

impl ProviderSettingsView {
    pub(crate) fn track_oauth_task(&mut self, handle: &tokio::task::JoinHandle<()>) {
        self.oauth_tasks.retain(|task| !task.is_finished());
        self.oauth_tasks.push(handle.abort_handle());
    }

    fn cancel_oauth_tasks(&mut self) {
        for task in self.oauth_tasks.drain(..) {
            task.abort();
        }
    }
}

impl Drop for ProviderSettingsView {
    fn drop(&mut self) {
        self.cancel_oauth_tasks();
    }
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

fn copy(text: String) -> ProviderSettingsEvent {
    ProviderSettingsEvent::Emit(Box::new(Action::CopyToClipboard(text)))
}

fn open(url: String) -> ProviderSettingsEvent {
    ProviderSettingsEvent::Emit(Box::new(Action::OAuthOpenUrl { url }))
}

/// The fields of `ProviderSettingsMode::OAuthHeadlessCodeEntry`.
struct CodeEntry {
    provider_id: String,
    authorize_url: String,
    pkce_verifier: String,
    code_input: String,
    code_focused: bool,
}

impl CodeEntry {
    fn into_mode(self) -> ProviderSettingsMode {
        ProviderSettingsMode::OAuthHeadlessCodeEntry {
            provider_id: self.provider_id,
            authorize_url: self.authorize_url,
            pkce_verifier: self.pkce_verifier,
            code_input: self.code_input,
            code_focused: self.code_focused,
        }
    }
}

/// TUI-126: a code character is printable ASCII (the GitHub Enterprise host
/// field uses the same gate).
pub(super) fn is_code_char(ch: char) -> bool {
    (' '..='~').contains(&ch)
}

/// The code-entry keyboard contract (anthropic headless):
///   * Esc              → cancel (generation bump, back to list)
///   * Enter            → submit ONLY when `code_input` is non-empty
///   * Tab / Up / Down  → move between the URL and the Code field
///   * on the URL: `c` copies it and `o` opens it without moving focus;
///     any other printable character or
///     Backspace moves to the Code field and edits the code
///   * in the Code field: every printable character, `c` and `o` included,
///     is typed into the code; Backspace deletes the last character
fn handle_code_entry_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    mut entry: CodeEntry,
) -> ProviderSettingsEvent {
    let event = match key.code {
        KeyCode::Esc => return cancel_to_list(view),
        KeyCode::Enter if !entry.code_input.is_empty() => {
            ProviderSettingsEvent::Emit(Box::new(Action::OAuthLoginHeadlessSubmit {
                provider_id: entry.provider_id.clone(),
                code: entry.code_input.clone(),
                pkce_verifier: entry.pkce_verifier.clone(),
                generation: view.oauth_generation,
            }))
        }
        KeyCode::Tab | KeyCode::BackTab => {
            entry.code_focused = !entry.code_focused;
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Down | KeyCode::Up => {
            entry.code_focused = key.code == KeyCode::Down;
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Char('c') if !entry.code_focused => copy(entry.authorize_url.clone()),
        KeyCode::Char('o') if !entry.code_focused => open(entry.authorize_url.clone()),
        KeyCode::Char(ch) if is_code_char(ch) => {
            entry.code_focused = true;
            entry.code_input.push(ch);
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Backspace => {
            entry.code_focused = true;
            entry.code_input.pop();
            ProviderSettingsEvent::Consumed
        }
        _ => ProviderSettingsEvent::Consumed,
    };
    // Rewrite the mode with the (possibly) updated input — Esc/Enter-submit
    // already returned above, so we always stay in code-entry here.
    view.mode = entry.into_mode();
    event
}

/// TUI-126: a paste on the code-entry screen goes into the Code field, with
/// line breaks and surrounding whitespace removed and only code characters
/// kept.
pub(super) fn paste_code(view: &mut ProviderSettingsView, text: &str) -> ProviderSettingsEvent {
    let ProviderSettingsMode::OAuthHeadlessCodeEntry {
        provider_id,
        authorize_url,
        pkce_verifier,
        mut code_input,
        ..
    } = view.mode.clone()
    else {
        return ProviderSettingsEvent::Ignored;
    };
    code_input.extend(text.trim().chars().filter(|ch| is_code_char(*ch)));
    view.mode = CodeEntry {
        provider_id,
        authorize_url,
        pkce_verifier,
        code_input,
        code_focused: true,
    }
    .into_mode();
    ProviderSettingsEvent::Consumed
}
