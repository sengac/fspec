//! RPC-106 — TS-parity footer hints with context-sensitive per-row-type strings.
//!
//! Feature: spec/features/rpc106-provider-settings-footer-hints.feature
//!
//! Pure dispatch module mirroring TypeScript `getFooterHints` from
//! `src/tui/utils/providerSettingsHelpers.ts:11-33`. The footer hint line
//! is composed from a per-row-kind prefix plus the shared `FOOTER_COMMON`
//! suffix, separated by U+00B7 MIDDLE DOT (`·`). Keybind labels use the
//! lowercase-colon style (`Enter:`, `d:`, `Esc:`), NOT the legacy
//! uppercase pipe-separated form (`Enter Select | D Delete`).
//!
//! This module is intentionally pure: `footer_hint_for(Option<RowKind>)`
//! is a function with no view state. Callers in `mod.rs` are responsible
//! for mapping the currently-focused `NavItem` to a `RowKind` and
//! passing it through.

use super::nav_item::NavItemKind;
use super::profile_form_render::FORM_FOOTER;
use super::row_render::RowKind;
use super::{DetailSub, ProviderSettingsMode, ProviderSettingsView};

/// The shared suffix appended to every per-row-kind hint. Verbatim copy
/// of the TS constant at `src/tui/utils/providerSettingsHelpers.ts:11`.
/// The separator is U+00B7 MIDDLE DOT.
pub const FOOTER_COMMON: &str = "/ filter · Tab: Switch to models · Esc: close";

/// Compute the full mode-sensitive footer hint for the view. List mode
/// dispatches on the focused row kind; Detail and form modes use their
/// dedicated strings.
pub(super) fn compute_footer_hint(view: &ProviderSettingsView) -> String {
    match &view.mode {
        ProviderSettingsMode::List => {
            footer_hint_for(focused_row_kind(view), focused_provider_configured(view))
        }
        ProviderSettingsMode::CreateProfile { .. }
        | ProviderSettingsMode::EditProfile { .. }
        | ProviderSettingsMode::EditSettings { .. } => FORM_FOOTER.to_string(),
        ProviderSettingsMode::DisconnectOAuth { .. } => "y: confirm · n/Esc: cancel".to_string(),
        ProviderSettingsMode::OAuthBrowserWaiting { .. } => "Esc: cancel".to_string(),
        // TUI-126: the hints follow the state and the focus.
        ProviderSettingsMode::OAuthDeviceWaiting { user_code, .. } => {
            if user_code.is_empty() {
                "Esc: cancel".to_string()
            } else {
                DEVICE_HINT.to_string()
            }
        }
        ProviderSettingsMode::OAuthHeadlessCodeEntry { code_focused, .. } => {
            code_entry_hint(*code_focused).to_string()
        }
        ProviderSettingsMode::OAuthSuccess { .. } => "Enter/Esc: continue".to_string(),
        ProviderSettingsMode::OAuthError { .. } => {
            "Enter: retry · c: copy error · Esc: back".to_string()
        }
        // PROV-114: the github-copilot device preamble modes.
        ProviderSettingsMode::OAuthDeploymentTypeSelect { .. } => {
            "↑/↓: select · Enter: continue · Esc: cancel".to_string()
        }
        ProviderSettingsMode::OAuthEnterpriseUrlEntry { .. } => {
            "Enter: continue · Esc: cancel".to_string()
        }
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { .. },
            ..
        } => "Enter: save · Esc: cancel".to_string(),
    }
}

/// TUI-126: the device-login keys once the user code is shown.
pub(super) const DEVICE_HINT: &str = "c: copy code · o: open URL · Esc: cancel";

/// TUI-126: the code-entry keys for the focused part (the URL or the Code
/// field). In the Code field `c` and `o` are code text, so they are not
/// offered.
pub(super) fn code_entry_hint(code_focused: bool) -> &'static str {
    if code_focused {
        "Enter: submit · Tab: back to the URL · Esc: cancel"
    } else {
        "c: copy URL · o: open URL · Tab: enter the code · Esc: cancel"
    }
}

/// Return the per-row-kind footer hint with `FOOTER_COMMON` appended.
///
/// When `kind` is `None` (no nav-item focused — empty list, post-filter
/// no-matches) the bare `FOOTER_COMMON` is returned, matching the TS
/// `default` branch.
///
/// TUI-132 (PS-15): a hint names only keys that act. `d` deletes a
/// Provider or ApiKey row's credentials only when the provider is
/// `configured`, so `d: delete` shows there only then; a Provider row
/// names its arrow key too.
///
/// | RowKind                 | Prefix                                   |
/// |-------------------------|------------------------------------------|
/// | Provider (collapsed)    | `Enter/→: expand` (+ `d: delete`)        |
/// | Provider (expanded)     | `Enter/←: collapse` (+ `d: delete`)      |
/// | OauthStatus             | `Enter: logout`                          |
/// | OauthLogin              | `Enter: start login`                     |
/// | ApiKey                  | `Enter: edit` (+ `d: delete`)            |
/// | Profile                 | `Enter: edit · d: delete`                |
/// | AddProfile              | `Enter: create`                          |
pub fn footer_hint_for(kind: Option<RowKind>, configured: bool) -> String {
    let delete = if configured { "d: delete · " } else { "" };
    match kind {
        None => FOOTER_COMMON.to_string(),
        Some(RowKind::Provider { expanded: false }) => {
            format!("Enter/→: expand · {delete}{FOOTER_COMMON}")
        }
        Some(RowKind::Provider { expanded: true }) => {
            format!("Enter/←: collapse · {delete}{FOOTER_COMMON}")
        }
        Some(RowKind::OauthStatus) => {
            format!("Enter: logout · {FOOTER_COMMON}")
        }
        Some(RowKind::OauthLogin) => {
            format!("Enter: start login · {FOOTER_COMMON}")
        }
        Some(RowKind::ApiKey) => {
            format!("Enter: edit · {delete}{FOOTER_COMMON}")
        }
        Some(RowKind::Profile) => {
            format!("Enter: edit · d: delete · {FOOTER_COMMON}")
        }
        Some(RowKind::AddProfile) => {
            format!("Enter: create · {FOOTER_COMMON}")
        }
        Some(RowKind::Settings) => format!("Enter: edit · {FOOTER_COMMON}"),
    }
}

/// TUI-132: whether the focused row's provider is configured (what `d`
/// on a Provider or ApiKey row needs before it opens the delete confirm).
fn focused_provider_configured(view: &ProviderSettingsView) -> bool {
    view.focused_nav_item().is_some_and(|item| {
        view.display_providers
            .iter()
            .any(|p| p.id == item.provider_id && p.configured)
    })
}

/// RPC-106: Translate the view's currently-focused NavItem into a `RowKind`
/// for the footer-hint dispatch (and any other future per-row callers).
/// Returns `None` when `nav_items` is empty.
pub(super) fn focused_row_kind(view: &ProviderSettingsView) -> Option<RowKind> {
    let item = view.nav_items.get(view.selected_index)?;
    Some(match &item.kind {
        NavItemKind::Provider { expanded } => RowKind::Provider {
            expanded: *expanded,
        },
        NavItemKind::Profile { .. } => RowKind::Profile,
        NavItemKind::Settings => RowKind::Settings,
        NavItemKind::AddProfile => RowKind::AddProfile,
        NavItemKind::ApiKey => RowKind::ApiKey,
        NavItemKind::OAuthLogin { .. } => RowKind::OauthLogin,
        NavItemKind::OAuthStatus { .. } => RowKind::OauthStatus,
    })
}
