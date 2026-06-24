//! PROV-113 — `ProviderSettingsMode` + `DetailSub` mode enums.
//!
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Extracted from `mod.rs` so the new OAuth-login modes can be added without
//! pushing that file past the 300-LoC ceiling. The OAuth login modes
//! (`OAuthBrowserWaiting`, `OAuthDeviceWaiting`, `OAuthHeadlessCodeEntry`,
//! `OAuthSuccess`, `OAuthError`) replace the dead-end `DetailSub::OAuthNotice`
//! placeholder for login rows (PROV-112 keeps `OAuthNotice` only for the
//! still-unwired github-copilot login, pending PROV-114).

use super::profile_form;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProviderSettingsMode {
    #[default]
    List,
    Detail {
        provider_id: String,
        sub: DetailSub,
    },
    /// PROV-110: create-profile form (name being authored from scratch).
    CreateProfile {
        provider_id: String,
        form: profile_form::ProfileForm,
    },
    /// PROV-110: edit-profile form (connection fields prefilled; name fixed).
    EditProfile {
        provider_id: String,
        profile_name: String,
        form: profile_form::ProfileForm,
    },
    /// PROV-112: dedicated disconnect/logout confirm for an oauth-status row,
    /// keyed by `provider_id`. `y`/`Y` emits `Action::OAuthDisconnect` then
    /// returns to list; `n`/`N`/Esc returns to list without any backend call;
    /// any other key is consumed (stays open).
    DisconnectOAuth {
        provider_id: String,
    },
    /// PROV-113: browser OAuth login is running fire-and-forget; the screen
    /// shows the provider's waiting title + "Waiting for authorization..." +
    /// "Press Esc to cancel". Esc increments the generation and returns to
    /// list (a late success/error is dropped).
    OAuthBrowserWaiting {
        provider_id: String,
    },
    /// PROV-113: codex device login is running; the screen shows the device
    /// waiting title + "Your code: <user_code>" + the verification URL + a
    /// spinner + "Press Esc to cancel". Esc cancels (generation bump).
    OAuthDeviceWaiting {
        provider_id: String,
        user_code: String,
        verification_url: String,
    },
    /// PROV-113: anthropic headless code entry. The authorize URL is shown
    /// immediately with a "Code:" input. While `code_input` is empty, `c`
    /// copies the URL and `o` opens it; once non-empty `c`/`o` are literal
    /// chars. Enter submits only when `code_input` is non-empty.
    OAuthHeadlessCodeEntry {
        provider_id: String,
        authorize_url: String,
        pkce_verifier: String,
        code_input: String,
    },
    /// PROV-113: a login succeeded — shows the provider success label +
    /// "Press Enter or Esc to continue". Enter/Esc returns to list.
    OAuthSuccess {
        provider_id: String,
    },
    /// PROV-113: a login failed — shows "OAuth Login error" + the message +
    /// "Press Enter to retry | Esc to go back". Enter retries the last method,
    /// Esc returns to list.
    OAuthError {
        provider_id: String,
        error: String,
    },
    /// PROV-114: github-copilot deployment-type preamble. Shows
    /// "GitHub Copilot Login — Select deployment type" with two options
    /// ("GitHub.com" index 0, "GitHub Enterprise" index 1). ↑/↓ move
    /// `selected_index`; Enter on 0 begins device polling (null host), Enter
    /// on 1 enters `OAuthEnterpriseUrlEntry`; Esc returns to list.
    OAuthDeploymentTypeSelect {
        provider_id: String,
        selected_index: usize,
    },
    /// PROV-114: github-copilot enterprise-host entry. Printable chars append
    /// to `url_input`; Backspace/Delete pop the last char and clear
    /// `validation_error`; Enter with empty input sets the validation error,
    /// Enter with a non-empty input normalizes the host and begins device
    /// polling; Esc returns to list.
    OAuthEnterpriseUrlEntry {
        provider_id: String,
        url_input: String,
        validation_error: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailSub {
    Summary {
        last_status: Option<super::DetailStatus>,
    },
    EditApiKey {
        draft: String,
    },
    OAuthNotice,
}
