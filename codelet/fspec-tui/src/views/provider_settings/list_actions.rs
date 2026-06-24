//! PROV-102 — List-mode Enter / `d` action dispatch keyed by the focused
//! NavItem identity.
//!
//! Feature: spec/features/provider-settings-nav-item-actions.feature
//!
//! Extracted from `list.rs` so the Enter / `d` match arms stay tiny and the
//! file stays under the 300-LoC ceiling. When the rich RPC-103 `nav_items`
//! tree is populated, the key handlers route here and we dispatch SOLELY on
//! the focused NavItem's `kind` + its own `provider_id`. We NEVER re-derive
//! the provider from `selected_index` against `visible_providers()` — that
//! index-space mismatch was the PROV-102 bug (an OpenAI profile row opened
//! Anthropic's Detail). The legacy `visible_providers()[selected_index]`
//! path in `list.rs` is now reachable only when `nav_items` is empty
//! (pre-RPC-103 `set_providers`-only callers), where the index is correct.
//!
//! TS reference: `src/tui/inputHandlers/listModeHandler.ts:118-177`.
//!
//! PROV-112: an OAuth *status* row (`OauthStatus`) now routes — on BOTH
//! `Enter` AND `d`/`D` — into the dedicated `ProviderSettingsMode::
//! DisconnectOAuth { provider_id }` confirm flow (it no longer falls into the
//! api-key delete-confirm path nor a silent no-op). OAuth *login* rows
//! (`OAuthLogin`) still route to the honest `DetailSub::OAuthNotice`
//! placeholder (keyed by the correct provider); wiring the real login flow is
//! pending PROV-113/114. The remaining parity gap is therefore narrower: the
//! Rust frontend still has no profile-create / profile-edit / OAuth-login
//! modes, so those rows route to `OAuthNotice` (login) or are consumed as
//! explicit no-ops (AddProfile, profile/add-profile/oauth-login `d`).

use crate::views::agent::confirm_dialog::ConfirmDialog;
use crate::views::provider_settings::nav_item::NavItemKind;
use crate::views::provider_settings::profile_form::ProfileForm;

use super::{DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// PROV-111: recover the bare profile name from a Profile-row label. Profile
/// rows carry the display string `"{name} → {baseUrl}"` (or just `"{name}"`
/// when there is no baseUrl); the edit-form prefill and per-profile delete
/// both key on the bare name, so split on the `" → "` separator and take the
/// leading segment.
fn bare_profile_name(display: &str) -> String {
    match display.split_once(" → ") {
        Some((name, _)) => name.to_string(),
        None => display.to_string(),
    }
}

/// Dispatch `Enter` for the focused NavItem. Every variant returns
/// `Consumed` (or a transition) — none fall through to the legacy path.
pub(super) fn enter_on_nav_item(
    view: &mut ProviderSettingsView,
    provider_id: String,
    kind: NavItemKind,
) -> ProviderSettingsEvent {
    match kind {
        // Provider header rows toggle expansion (selected_index untouched).
        NavItemKind::Provider { .. } => {
            view.toggle_expansion(&provider_id);
            ProviderSettingsEvent::Consumed
        }
        // ApiKey rows open the inline edit form.
        NavItemKind::ApiKey => {
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::EditApiKey {
                    draft: String::new(),
                },
            };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        // PROV-111: Enter on a Profile row opens the EditProfile form
        // prefilled from the FULL stored ProfileConfig (looked up by the
        // row's bare name, recovered from its "{name} → {baseUrl}" display
        // label). A missing config degrades to an empty definition so the
        // form still opens (the name stays fixed/non-editable). Replaces the
        // old read-only Detail/Summary placeholder (the blank-screen bug).
        NavItemKind::Profile { profile_name } => {
            let name = bare_profile_name(&profile_name);
            let form = match view.profile_config_for(&name) {
                Some(def) => ProfileForm::from_definition(&name, def),
                None => ProfileForm::from_definition(&name, &Default::default()),
            };
            view.mode = ProviderSettingsMode::EditProfile {
                provider_id,
                profile_name: name,
                form,
            };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        // PROV-112: Enter on an oauth-status (Logout) row opens the dedicated
        // DisconnectOAuth confirm keyed by this row's provider_id — NOT the
        // generic OAuthNotice placeholder and NOT the api-key delete confirm.
        NavItemKind::OAuthStatus { .. } => {
            view.mode = ProviderSettingsMode::DisconnectOAuth { provider_id };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        // PROV-113/114: Enter on an OAuth login row starts the real login
        // flow. The github-copilot row routes FIRST (by provider_id, before
        // method) into the PROV-114 deployment-type preamble; every other
        // provider routes by (provider, method) in `oauth_login`.
        NavItemKind::OAuthLogin { method, .. } => {
            if super::oauth_copilot::is_copilot(&provider_id) {
                super::oauth_copilot::start_deployment_select(view, provider_id)
            } else {
                super::oauth_login::start_oauth_login(view, provider_id, method)
            }
        }
        // PROV-111: Enter on the AddProfile row opens the CreateProfile form
        // (TS initializeNewProfile): empty name being edited, default base
        // URL. Replaces the old explicit no-op.
        NavItemKind::AddProfile => {
            view.mode = ProviderSettingsMode::CreateProfile {
                provider_id,
                form: ProfileForm::new_create(),
            };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
    }
}

/// Dispatch `d` for the focused NavItem. Provider / ApiKey / OAuthStatus
/// rows open the delete-credentials confirm for the row's own provider
/// (TS maps api-key→delete-api-key, oauth-status→disconnect-oauth; the Rust
/// port collapses both to the single delete-credentials confirm, and keeps
/// the provider-row delete affordance). Profile / AddProfile / OAuthLogin
/// rows have no delete action (TS `d` is a no-op there, and the Rust port
/// has no per-profile delete) so they are consumed without mis-selecting.
pub(super) fn delete_on_nav_item(view: &mut ProviderSettingsView) -> ProviderSettingsEvent {
    let Some(item) = view.focused_nav_item() else {
        return ProviderSettingsEvent::Consumed;
    };
    let provider_id = item.provider_id.clone();
    match &item.kind {
        NavItemKind::Provider { .. } | NavItemKind::ApiKey => {
            open_delete_confirm(view, &provider_id)
        }
        // PROV-112: `d`/`D` on an oauth-status (Logout) row opens the SAME
        // dedicated DisconnectOAuth confirm as Enter (TS maps oauth-status `d`
        // identically to Enter → disconnect-oauth). It must NOT collapse into
        // the generic delete-credentials confirm.
        NavItemKind::OAuthStatus { .. } => {
            view.mode = ProviderSettingsMode::DisconnectOAuth { provider_id };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        // PROV-111: `d` on a Profile row opens a per-profile delete-confirm.
        // The Primary acceptance emits ConfirmDeleteProfile for ONLY this
        // providers.<provider>.profiles.<name> key (see mod.rs handle_key).
        NavItemKind::Profile { profile_name } => {
            let name = bare_profile_name(profile_name);
            open_profile_delete_confirm(view, provider_id, name)
        }
        NavItemKind::AddProfile | NavItemKind::OAuthLogin { .. } => ProviderSettingsEvent::Consumed,
    }
}

/// PROV-111: open the per-profile delete ConfirmDialog and record the pending
/// `(provider_id, profile_name)` target so the shared `delete_confirm` Primary
/// arm in `handle_key` emits `ConfirmDeleteProfile` for it.
fn open_profile_delete_confirm(
    view: &mut ProviderSettingsView,
    provider_id: String,
    profile_name: String,
) -> ProviderSettingsEvent {
    let body = format!("Delete profile {profile_name}?");
    view.delete_confirm = Some(ConfirmDialog::new(
        "Delete profile?",
        body,
        "Delete",
        None,
        "Cancel",
    ));
    view.pending_profile_delete = Some((provider_id, profile_name));
    ProviderSettingsEvent::Consumed
}

/// Open the delete-credentials ConfirmDialog for `provider_id`, gated on the
/// provider being configured (mirrors the legacy `focused.configured` gate).
/// The `configured` flag is read from `display_providers` by id, NOT by
/// indexing with `selected_index`.
fn open_delete_confirm(
    view: &mut ProviderSettingsView,
    provider_id: &str,
) -> ProviderSettingsEvent {
    let configured = view
        .display_providers
        .iter()
        .find(|p| p.id == provider_id)
        .map(|p| p.configured)
        .unwrap_or(false);
    if !configured {
        return ProviderSettingsEvent::Consumed;
    }
    let body = format!("Delete credentials for {provider_id}?");
    view.delete_confirm = Some(ConfirmDialog::new(
        "Delete credentials?",
        body,
        "Delete",
        None,
        "Cancel",
    ));
    ProviderSettingsEvent::Consumed
}
