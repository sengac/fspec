//! PROV-102 — List-mode Enter / d dispatch by focused NavItem identity.
//!
//! Feature: spec/features/provider-settings-nav-item-actions.feature
//!
//! Regression coverage for the index-space mismatch bug: when the rich
//! RPC-103 NavItem tree is populated, `selected_index` is a `nav_items`
//! index, but the legacy Enter / `d` fallthrough re-indexed
//! `visible_providers()` (top-level providers only) with it. On child rows
//! this either resolved to a DIFFERENT provider (e.g. an OpenAI profile row
//! opening Anthropic's Detail) or returned `None` (a silent no-op).
//!
//! These tests build a MULTI-provider expanded tree so the mismatched index
//! would mis-select, then assert the opened Detail / delete action targets
//! the focused row's OWN provider_id. All tests are pure view-layer drives
//! (no App, no backend, no tokio) — fully offline.

use codelet_test_helpers::{OrFail, TestResult};
use std::collections::HashMap;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn openai_row_index(
    view: &ProviderSettingsView,
    kind: NavItemKind,
) -> Result<usize, codelet_test_helpers::TestError> {
    Ok(view
        .nav_items
        .iter()
        .position(|item| item.provider_id == "openai" && item.kind == kind)
        .or_fail("requested OpenAI navigation row must be present")?)
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
        masked_key: None,
        source: None,
    }
}

/// Build a view loaded with openai, anthropic, gemini via BOTH the legacy
/// `set_providers` list (the mis-select target) AND the rich
/// `set_provider_display_infos` nav tree (the navigated/rendered list).
/// openai carries a single custom profile "fast".
fn multi_provider_view() -> ProviderSettingsView {
    let creds = vec![
        pinfo("openai", "api_key", true, 5),
        pinfo("anthropic", "api_key", true, 8),
        pinfo("gemini", "api_key", true, 3),
    ];
    let profiles = vec!["fast".to_string()];
    let display = project_display_infos(&creds, &profiles);
    let mut view = ProviderSettingsView::new();
    view.set_providers(creds);
    view.set_provider_display_infos(display);
    // PROV-111: seed the per-profile config map so Enter on a Profile row
    // can open the EditProfile form prefilled from the stored config.
    let mut configs = HashMap::new();
    configs.insert(
        ("openai".to_string(), "fast".to_string()),
        ProfileDefinition {
            base_url: "http://localhost:9001".to_string(),
            api_key: "sk-fast".to_string(),
            context_window: None,
            max_output_tokens: None,
            compaction_threshold_type: None,
            compaction_threshold_value: None,
            streaming: None,
            auto_continue: None,
            preserve_thinking: None,
            max_images: None,
            loop_detection_enabled: None,
            loop_detection_window: None,
            loop_detection_max_repeats: None,
            loop_detection_max_retries: None,
        },
    );
    view.set_profile_configs(configs);
    view
}

fn focused_kind(
    view: &ProviderSettingsView,
) -> Result<NavItemKind, codelet_test_helpers::TestError> {
    Ok(view
        .focused_nav_item()
        .or_fail("a nav item must be focused")?
        .kind
        .clone())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on an OpenAI profile row opens the OpenAI EDIT form, not
// Anthropic (PROV-111: the row now opens the prefilled EditProfile form
// instead of the old read-only Detail/Summary placeholder).
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_openai_profile_opens_openai_edit_form_not_anthropic() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the openai provider is expanded so its "fast" profile row is visible
    // (cursor starts on the openai provider row at index 0; Enter expands it)
    assert!(matches!(focused_kind(&view)?, NavItemKind::Provider { .. }));
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on the openai "fast" profile row
    view.selected_index = openai_row_index(
        &view,
        NavItemKind::Profile {
            profile_name: "fast".to_string(),
        },
    )?;
    match focused_kind(&view)? {
        NavItemKind::Profile { profile_name } => assert_eq!(profile_name, "fast"),
        other => return Err(format!("expected openai 'fast' profile row, got {other:?}").into()),
    }

    // @step When I press Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the EditProfile form that opens has provider_id "openai"
    // @step And the form is prefilled from the stored "fast" config
    // @step And the form's provider_id is not "anthropic"
    match &view.mode {
        ProviderSettingsMode::EditProfile {
            provider_id,
            profile_name,
            form,
        } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "fast");
            assert_eq!(form.base_url, "http://localhost:9001");
            assert_eq!(form.api_key, "sk-fast");
            assert_ne!(provider_id, "anthropic");
        }
        other => return Err(format!("expected EditProfile mode for openai, got {other:?}").into()),
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on an OAuthStatus row opens the DisconnectOAuth confirm for
// its own provider, not a mismatched one (PROV-112)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_oauth_status_opens_disconnect_confirm_for_own_provider() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the anthropic provider is expanded so its OAuthStatus row is visible
    view.handle_key(key(KeyCode::Down));
    assert!(matches!(focused_kind(&view)?, NavItemKind::Provider { .. }));
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on the anthropic OAuthStatus row
    view.handle_key(key(KeyCode::Down));
    assert!(matches!(
        focused_kind(&view)?,
        NavItemKind::OAuthStatus { .. }
    ));

    // @step When I press Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the DisconnectOAuth confirm that opens has provider_id "anthropic"
    // @step And the provider_id is not "gemini"
    // (PROV-112 superseded the old OAuthNotice placeholder: Enter on an
    // oauth-status row now opens the dedicated DisconnectOAuth confirm keyed by
    // the focused row's OWN provider — the anti-mismatch invariant is kept.)
    match &view.mode {
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            assert_eq!(provider_id, "anthropic");
            assert_ne!(provider_id, "gemini");
        }
        other => {
            return Err(format!("expected DisconnectOAuth for anthropic, got {other:?}").into())
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on an OAuthLogin row starts the real login flow instead of
// silently doing nothing.
//
// PROV-113 wires the real flow: Enter emits `Action::OAuthLoginStart` keyed by the row's
// own provider (a Browser row also enters `OAuthBrowserWaiting` immediately; a
// Headless/Device row leaves the mode to the dispatch start-result). The
// anti-noop / anti-mismatch invariant is preserved — Enter is never a silent
// no-op and always targets the focused row's OWN provider.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_oauth_login_starts_login_flow_not_silent_noop() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the anthropic provider is expanded so its OAuthLogin row is visible
    view.handle_key(key(KeyCode::Down));
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on an anthropic OAuthLogin row
    view.handle_key(key(KeyCode::Down)); // OAuthStatus
    view.handle_key(key(KeyCode::Down)); // OAuthLogin
    assert!(matches!(
        focused_kind(&view)?,
        NavItemKind::OAuthLogin { .. }
    ));

    // @step When I press Enter
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the login flow starts for provider_id "anthropic"
    match event {
        ProviderSettingsEvent::Emit(action) => {
            let Action::OAuthLoginStart { provider_id, .. } = *action else {
                return Err(format!(
                    "expected Emit(OAuthLoginStart) for anthropic, got {action:?}"
                )
                .into());
            };
            assert_eq!(provider_id, "anthropic");
        }
        other => {
            return Err(
                format!("expected Emit(OAuthLoginStart) for anthropic, got {other:?}").into(),
            )
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on the AddProfile row opens the create form
// (PROV-111: the row now opens CreateProfile instead of being a no-op).
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_add_profile_opens_create_form() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the openai provider is expanded so its AddProfile row is visible
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on the openai AddProfile row
    view.selected_index = openai_row_index(&view, NavItemKind::AddProfile)?;
    assert!(matches!(focused_kind(&view)?, NavItemKind::AddProfile));

    // @step When I press Enter
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then the view enters CreateProfile mode for "openai"
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    match &view.mode {
        ProviderSettingsMode::CreateProfile { provider_id, form } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(form.base_url, "http://localhost:8888");
            assert!(form.is_editing_name);
        }
        other => {
            return Err(format!("expected CreateProfile mode for openai, got {other:?}").into())
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: d on an OAuthStatus row opens the DisconnectOAuth confirm for that
// provider, not a mismatched one (PROV-112 superseded the old generic
// delete-credentials confirm: `d` on oauth-status now mirrors Enter →
// DisconnectOAuth, keyed by the row's OWN provider).
// ────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_oauth_status_opens_disconnect_for_own_provider_not_mismatched() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the anthropic provider is expanded so its OAuthStatus row is visible
    view.handle_key(key(KeyCode::Down));
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on the anthropic OAuthStatus row
    view.handle_key(key(KeyCode::Down));
    assert!(matches!(
        focused_kind(&view)?,
        NavItemKind::OAuthStatus { .. }
    ));

    // @step When I press d
    view.handle_key(key(KeyCode::Char('d')));

    // @step Then the generic delete-credentials confirm is not opened
    assert!(
        view.delete_confirm.is_none(),
        "`d` on oauth-status must NOT open the generic delete-credentials confirm"
    );

    // @step And the DisconnectOAuth confirm targets "anthropic", not "gemini"
    match &view.mode {
        ProviderSettingsMode::DisconnectOAuth { provider_id } => {
            assert_eq!(provider_id, "anthropic");
            assert_ne!(provider_id, "gemini");
        }
        other => {
            return Err(format!("expected DisconnectOAuth for anthropic, got {other:?}").into())
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: d on a profile row opens the per-profile delete confirm
// (PROV-111: profile rows now have a per-profile delete action; the dialog's
// Primary acceptance emits ConfirmDeleteProfile, NOT the provider-credentials
// delete).
// ────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_profile_row_opens_per_profile_delete_confirm() -> TestResult {
    // @step Given the Provider Settings nav tree is loaded with openai, anthropic and gemini
    let mut view = multi_provider_view();

    // @step And the openai provider is expanded so its "fast" profile row is visible
    view.handle_key(key(KeyCode::Enter));

    // @step And the cursor is on the openai "fast" profile row
    view.selected_index = openai_row_index(
        &view,
        NavItemKind::Profile {
            profile_name: "fast".to_string(),
        },
    )?;
    match focused_kind(&view)? {
        NavItemKind::Profile { profile_name } => assert_eq!(profile_name, "fast"),
        other => return Err(format!("expected openai 'fast' profile row, got {other:?}").into()),
    }

    // @step When I press d
    view.handle_key(key(KeyCode::Char('d')));

    // @step Then a per-profile delete confirmation dialog is open
    assert!(view.delete_confirm.is_some());

    // @step And accepting it emits ConfirmDeleteProfile for openai profile "fast"
    let out = view.handle_key(key(KeyCode::Enter));
    match out {
        ProviderSettingsEvent::Emit(action) => {
            let Action::ConfirmDeleteProfile {
                provider_id,
                profile_name,
            } = *action
            else {
                return Err(format!(
                    "expected Emit(ConfirmDeleteProfile{{openai,fast}}), got {action:?}"
                )
                .into());
            };
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "fast");
        }
        other => {
            return Err(format!(
                "expected Emit(ConfirmDeleteProfile{{openai,fast}}), got {other:?}"
            )
            .into())
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Legacy set_providers callers keep their pre-existing d behavior
// ────────────────────────────────────────────────────────────────────────

#[test]
fn legacy_set_providers_callers_keep_d_behavior() -> TestResult {
    // @step Given a Provider Settings view populated only via set_providers with a configured api_key provider
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("anthropic", "api_key", true, 8)]);

    // @step And the nav tree is empty
    assert!(view.nav_items.is_empty());

    // @step When I press d on the configured provider row
    view.handle_key(key(KeyCode::Char('d')));

    // @step Then a delete confirmation dialog is open
    assert!(view.delete_confirm.is_some());
    Ok(())
}
