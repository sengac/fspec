//! RPC-349 — Provider Settings rich NavItem tree wiring.
//!
//! Feature: spec/features/provider-settings-rich-tree.feature
//!
//! Regression coverage for the bug where the live `/provider` dispatch
//! path folded the loaded credentials via `set_providers()` only — which
//! never populated `display_providers` nor rebuilt `nav_items` — so the
//! rich RPC-103 NavItem tree (provider expansion, API-key edit rows,
//! OAuth login/logout rows, OpenAI profiles) was dead code at runtime and
//! the screen fell back to the legacy flat list.
//!
//! These tests pin BOTH layers:
//!   * the pure `project_display_infos` projection (ProviderCredentialInfo
//!     -> ProviderDisplayInfo), and
//!   * the end-to-end App dispatch wiring (Action::ProviderCredentialsLoaded
//!     now drives `set_provider_display_infos`, so `nav_items` is non-empty).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{DetailSub, ProviderSettingsMode, ProviderSettingsView};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

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

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

fn count_kind(
    view: &ProviderSettingsView,
    provider_id: &str,
    pred: impl Fn(&NavItemKind) -> bool,
) -> usize {
    view.nav_items
        .iter()
        .filter(|i| i.provider_id == provider_id && pred(&i.kind))
        .count()
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Loading credentials populates the rich nav tree, not the legacy flat list
// ────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loading_credentials_populates_rich_nav_tree() {
    // @step Given the backend returns provider credentials for "openai", "anthropic", and "gemini"
    let mock = Arc::new(MockBackend::new());
    mock.seed_provider_credentials(vec![
        pinfo("openai", "api_key", false, 4),
        pinfo("anthropic", "oauth", true, 8),
        pinfo("gemini", "api_key", false, 6),
    ]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When the Provider Settings view folds the loaded credentials
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Provider));
    drain_pending(&mut app).await;

    // @step Then the view's nav tree contains three collapsed provider rows
    let view = &app.navigator().provider_settings;
    let provider_rows = view
        .nav_items
        .iter()
        .filter(|i| matches!(i.kind, NavItemKind::Provider { expanded: false }))
        .count();
    assert_eq!(
        provider_rows, 3,
        "expected 3 collapsed provider rows, got {provider_rows}"
    );

    // @step And the header item count reports 3 items
    assert!(
        view.title_text().contains("3 items"),
        "header should report 3 items, got {:?}",
        view.title_text()
    );

    // @step And the legacy "(no providers configured)" placeholder is not used
    // (a non-empty nav_items forces render_list down the render_nav_items
    //  branch, so the placeholder is unreachable)
    assert!(!view.nav_items.is_empty(), "nav_items must be populated");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Expanding an api-key provider reveals an editable API-key row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_api_key_provider_reveals_editable_api_key_row() {
    // @step Given the Provider Settings view has loaded a "gemini" provider of credential type "api_key"
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(
        &[pinfo("gemini", "api_key", false, 6)],
        &[],
    ));

    // @step When the "gemini" provider row is expanded
    view.toggle_expansion("gemini");

    // @step Then an ApiKey row appears beneath the "gemini" provider
    assert_eq!(
        count_kind(&view, "gemini", |k| matches!(k, NavItemKind::ApiKey)),
        1,
        "expected exactly one ApiKey row under gemini"
    );

    // Focus the ApiKey row (index 1 — provider row is index 0).
    view.selected_index = 1;
    assert!(matches!(
        view.focused_nav_item().map(|i| &i.kind),
        Some(NavItemKind::ApiKey)
    ));

    // @step When Enter is pressed on the ApiKey row
    let _ = view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // @step Then the view enters the EditApiKey detail sub-view for "gemini"
    assert!(matches!(
        &view.mode,
        ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::EditApiKey { .. },
        } if provider_id == "gemini"
    ));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Expanding a configured OAuth provider reveals logout and login rows
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_configured_oauth_provider_reveals_logout_and_login_rows() {
    // @step Given the Provider Settings view has loaded an "anthropic" provider of credential type "oauth" that is configured
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(
        &[pinfo("anthropic", "oauth", true, 8)],
        &[],
    ));

    // @step When the "anthropic" provider row is expanded
    view.toggle_expansion("anthropic");

    // @step Then a "Logout from OAuth [Anthropic]" row appears beneath the "anthropic" provider
    let has_logout = view.nav_items.iter().any(|i| {
        i.provider_id == "anthropic"
            && matches!(&i.kind, NavItemKind::OAuthStatus { label } if label.contains("Logout from OAuth"))
    });
    assert!(has_logout, "expected a logout row under anthropic");

    // @step And one or more OAuth login rows appear beneath the "anthropic" provider
    assert!(
        count_kind(&view, "anthropic", |k| matches!(
            k,
            NavItemKind::OAuthLogin { .. }
        )) >= 1,
        "expected at least one OAuth login row under anthropic"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Expanding an uncredentialed OAuth provider reveals only login rows
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_uncredentialed_oauth_provider_reveals_only_login_rows() {
    // @step Given the Provider Settings view has loaded a "codex" provider of credential type "oauth" that is not configured
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(
        &[pinfo("codex", "oauth", false, 0)],
        &[],
    ));

    // @step When the "codex" provider row is expanded
    view.toggle_expansion("codex");

    // @step Then no logout row appears beneath the "codex" provider
    assert_eq!(
        count_kind(&view, "codex", |k| matches!(
            k,
            NavItemKind::OAuthStatus { .. }
        )),
        0,
        "uncredentialed provider must not show a logout row"
    );

    // @step And one or more OAuth login rows appear beneath the "codex" provider
    assert!(
        count_kind(&view, "codex", |k| matches!(
            k,
            NavItemKind::OAuthLogin { .. }
        )) >= 1,
        "expected at least one OAuth login row under codex"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Expanding the openai provider reveals profiles and an add-profile row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn expanding_openai_provider_reveals_profiles_and_add_profile_row() {
    // @step Given the Provider Settings view has loaded an "openai" provider with profiles "fast" and "local"
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(
        &[pinfo("openai", "api_key", true, 4)],
        &["fast".to_string(), "local".to_string()],
    ));

    // @step When the "openai" provider row is expanded
    view.toggle_expansion("openai");

    // @step Then a Profile row appears for "fast"
    assert!(
        view.nav_items.iter().any(
            |i| matches!(&i.kind, NavItemKind::Profile { profile_name } if profile_name == "fast")
        ),
        "expected a Profile row for 'fast'"
    );

    // @step And a Profile row appears for "local"
    assert!(
        view.nav_items.iter().any(
            |i| matches!(&i.kind, NavItemKind::Profile { profile_name } if profile_name == "local")
        ),
        "expected a Profile row for 'local'"
    );

    // @step And a trailing "Add Profile" row appears beneath the "openai" profiles
    assert_eq!(
        count_kind(&view, "openai", |k| matches!(k, NavItemKind::AddProfile)),
        1,
        "expected exactly one AddProfile row under openai"
    );

    // @step And no ApiKey row appears beneath the "openai" provider
    assert_eq!(
        count_kind(&view, "openai", |k| matches!(k, NavItemKind::ApiKey)),
        0,
        "openai must never get an ApiKey row"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Header item count grows when a provider is expanded
// ────────────────────────────────────────────────────────────────────────

#[test]
fn header_item_count_grows_when_provider_is_expanded() {
    // @step Given the Provider Settings view has loaded a "gemini" provider of credential type "api_key"
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(
        &[pinfo("gemini", "api_key", false, 6)],
        &[],
    ));

    // @step And the header item count reports 1 item
    assert!(
        view.title_text().contains("1 items"),
        "collapsed: header should report 1 item, got {:?}",
        view.title_text()
    );

    // @step When the "gemini" provider row is expanded
    view.toggle_expansion("gemini");

    // @step Then the header item count reports 2 items
    assert!(
        view.title_text().contains("2 items"),
        "expanded: header should report 2 items, got {:?}",
        view.title_text()
    );
}
