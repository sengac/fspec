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
use super::row_render::RowKind;
use super::ProviderSettingsView;

/// The shared suffix appended to every per-row-kind hint. Verbatim copy
/// of the TS constant at `src/tui/utils/providerSettingsHelpers.ts:11`.
/// The separator is U+00B7 MIDDLE DOT.
pub const FOOTER_COMMON: &str = "/ filter · Tab: Switch to models · Esc: close";

/// Return the per-row-kind footer hint with `FOOTER_COMMON` appended.
///
/// When `kind` is `None` (no nav-item focused — empty list, post-filter
/// no-matches) the bare `FOOTER_COMMON` is returned, matching the TS
/// `default` branch.
///
/// Mirrors the switch table in `getFooterHints(itemType)`:
///
/// | RowKind         | Prefix                |
/// |-----------------|-----------------------|
/// | Provider        | `Enter: expand`       |
/// | OauthStatus     | `Enter: logout`       |
/// | OauthLogin      | `Enter: start login`  |
/// | ApiKey          | `Enter: edit · d: delete` |
/// | Profile         | `Enter: edit · d: delete` |
/// | AddProfile      | `Enter: create`       |
pub fn footer_hint_for(kind: Option<RowKind>) -> String {
    match kind {
        None => FOOTER_COMMON.to_string(),
        Some(RowKind::Provider { .. }) => {
            format!("Enter: expand · {FOOTER_COMMON}")
        }
        Some(RowKind::OauthStatus) => {
            format!("Enter: logout · {FOOTER_COMMON}")
        }
        Some(RowKind::OauthLogin) => {
            format!("Enter: start login · {FOOTER_COMMON}")
        }
        Some(RowKind::ApiKey) => {
            format!("Enter: edit · d: delete · {FOOTER_COMMON}")
        }
        Some(RowKind::Profile) => {
            format!("Enter: edit · d: delete · {FOOTER_COMMON}")
        }
        Some(RowKind::AddProfile) => {
            format!("Enter: create · {FOOTER_COMMON}")
        }
    }
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
        NavItemKind::AddProfile => RowKind::AddProfile,
        NavItemKind::ApiKey => RowKind::ApiKey,
        NavItemKind::OAuthLogin { .. } => RowKind::OauthLogin,
        NavItemKind::OAuthStatus { .. } => RowKind::OauthStatus,
    })
}
