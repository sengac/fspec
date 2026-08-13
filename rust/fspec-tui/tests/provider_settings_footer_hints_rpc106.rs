//! RPC-106 — TS-parity footer hints with context-sensitive per-row-type strings (failing red tests).
//!
//! Feature: spec/features/rpc106-provider-settings-footer-hints.feature
//!
//! Validates the new `footer_hints` module + the `ProviderSettingsView::footer_hint()`
//! wrapper that maps the currently-focused `NavItem` to a `RowKind` and renders
//! the per-row-type hint string with bullet (·) separators and lowercase-colon
//! keybind labels — exactly mirroring the TS `getFooterHints` reference at
//! `src/tui/utils/providerSettingsHelpers.ts:16-33`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_fspec_tui::views::provider_settings::footer_hints::{footer_hint_for, FOOTER_COMMON};
use codelet_fspec_tui::views::provider_settings::nav_item::{OAuthMethod, ProviderDisplayInfo};
use codelet_fspec_tui::views::provider_settings::row_render::RowKind;
use codelet_fspec_tui::views::ProviderSettingsView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

/// The canonical FOOTER_COMMON literal — must match TS verbatim
/// (`src/tui/utils/providerSettingsHelpers.ts:11`).
const EXPECTED_FOOTER_COMMON: &str = "/ filter · Tab: Switch to models · Esc: close";

fn openai_with_profiles(profiles: &[&str]) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        configured: !profiles.is_empty(),
        credential_type: "api_key".to_string(),
        model_count: 4,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: profiles.iter().map(ToString::to_string).collect(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

fn anthropic_oauth() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured: false,
        credential_type: "oauth".to_string(),
        model_count: 8,
        has_oauth_tokens: true,
        is_oauth_provider: true,
        requires_api_key: true,
        env_var: Some("ANTHROPIC_API_KEY".to_string()),
        profiles: Vec::new(),
        oauth_login_methods: vec![(OAuthMethod::Browser, "Sign in with browser".to_string())],
        oauth_status_label: Some("Logout from OAuth [Anthropic]".to_string()),
        masked_key: None,
        source: None,
    }
}

fn view_with(provider: ProviderDisplayInfo, expanded_ids: &[&str]) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![provider]);
    let mut expanded: HashSet<String> = HashSet::new();
    for id in expanded_ids {
        expanded.insert((*id).to_string());
    }
    view.expanded = expanded;
    view.rebuild_nav_items();
    view
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Provider row selection yields the "Enter: expand" hint with
//           FOOTER_COMMON suffix
// ────────────────────────────────────────────────────────────────────────

#[test]
fn provider_row_yields_enter_expand_hint() {
    // @step Given the ProviderSettingsView has a Provider nav-item selected
    let mut view = view_with(openai_with_profiles(&[]), &[]);
    view.selected_index = 0;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: expand · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: expand · / filter · Tab: Switch to models · Esc: close",
        "provider-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: ApiKey row selection yields the "Enter: edit · d: delete" hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn api_key_row_yields_enter_edit_d_delete_hint() {
    // @step Given the ProviderSettingsView has an ApiKey nav-item selected
    // (anthropic expanded → row 0 provider, row 1 oauth-status, row 2 oauth-login,
    //  row 3 api-key)
    let mut view = view_with(anthropic_oauth(), &["anthropic"]);
    // Walk down to the ApiKey row.
    let api_key_idx = view
        .nav_items
        .iter()
        .position(|n| {
            matches!(
                n.kind,
                codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::ApiKey
            )
        })
        .expect("anthropic-with-oauth must produce an ApiKey row");
    view.selected_index = api_key_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close",
        "api-key-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Profile row selection yields the "Enter: edit · d: delete" hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn profile_row_yields_enter_edit_d_delete_hint() {
    // @step Given the ProviderSettingsView has a Profile nav-item selected
    let mut view = view_with(openai_with_profiles(&["prof1"]), &["openai"]);
    let profile_idx = view
        .nav_items
        .iter()
        .position(|n| {
            matches!(
                &n.kind,
                codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::Profile { .. }
            )
        })
        .expect("openai with one profile must produce a Profile row");
    view.selected_index = profile_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close",
        "profile-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: AddProfile row selection yields the "Enter: create" hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn add_profile_row_yields_enter_create_hint() {
    // @step Given the ProviderSettingsView has an AddProfile nav-item selected
    let mut view = view_with(openai_with_profiles(&[]), &["openai"]);
    let add_idx = view
        .nav_items
        .iter()
        .position(|n| {
            matches!(
                n.kind,
                codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::AddProfile
            )
        })
        .expect("expanded openai must produce an AddProfile row");
    view.selected_index = add_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: create · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: create · / filter · Tab: Switch to models · Esc: close",
        "add-profile-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: OAuthLogin row selection yields the "Enter: start login" hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn oauth_login_row_yields_enter_start_login_hint() {
    // @step Given the ProviderSettingsView has an OAuthLogin nav-item selected
    let mut view = view_with(anthropic_oauth(), &["anthropic"]);
    // PROV-113: the `anthropic_oauth()` fixture declares only a Browser login
    // method, and browser rows are now gated out of the nav tree unless the
    // transport supports the local OAuth server. Enable it so an OAuthLogin
    // row exists to focus (the hint is identical for browser/headless rows).
    view.set_browser_login_enabled(true);
    let login_idx =
        view.nav_items
            .iter()
            .position(|n| {
                matches!(
            &n.kind,
            codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::OAuthLogin { .. }
        )
            })
            .expect("anthropic expanded must produce an OAuthLogin row");
    view.selected_index = login_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: start login · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: start login · / filter · Tab: Switch to models · Esc: close",
        "oauth-login-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: OAuthStatus row selection yields the "Enter: logout" hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn oauth_status_row_yields_enter_logout_hint() {
    // @step Given the ProviderSettingsView has an OAuthStatus nav-item selected
    let mut view = view_with(anthropic_oauth(), &["anthropic"]);
    let status_idx =
        view.nav_items
            .iter()
            .position(|n| {
                matches!(
            &n.kind,
            codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::OAuthStatus { .. }
        )
            })
            .expect("anthropic-with-oauth-tokens expanded must produce an OAuthStatus row");
    view.selected_index = status_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "Enter: logout · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        hint, "Enter: logout · / filter · Tab: Switch to models · Esc: close",
        "oauth-status-row hint mismatch"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Empty nav-item list (no selection) falls back to FOOTER_COMMON only
// ────────────────────────────────────────────────────────────────────────

#[test]
fn empty_nav_items_yields_footer_common_only() {
    // @step Given the ProviderSettingsView has no nav-items
    let view = ProviderSettingsView::new();
    assert!(
        view.nav_items.is_empty(),
        "precondition: nav_items must be empty"
    );

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint equals "/ filter · Tab: Switch to models · Esc: close"
    assert_eq!(hint, EXPECTED_FOOTER_COMMON, "empty-nav fallback mismatch");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Separator is U+00B7 MIDDLE DOT, never the pipe character
// ────────────────────────────────────────────────────────────────────────

#[test]
fn separator_is_middle_dot_never_pipe() {
    // @step Given the ProviderSettingsView has a Provider nav-item selected
    let mut view = view_with(openai_with_profiles(&[]), &[]);
    view.selected_index = 0;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint contains the character "·" (U+00B7)
    assert!(
        hint.contains('\u{00B7}'),
        "hint must contain U+00B7 MIDDLE DOT: {hint:?}"
    );
    // @step And the hint does not contain the character "|"
    assert!(
        !hint.contains('|'),
        "hint must NOT contain pipe '|': {hint:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Keybind labels use lowercase colon style, never uppercase pipe style
// ────────────────────────────────────────────────────────────────────────

#[test]
fn keybind_labels_use_lowercase_colon_style() {
    // @step Given the ProviderSettingsView has an ApiKey nav-item selected
    let mut view = view_with(anthropic_oauth(), &["anthropic"]);
    let api_key_idx = view
        .nav_items
        .iter()
        .position(|n| {
            matches!(
                n.kind,
                codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind::ApiKey
            )
        })
        .expect("anthropic-with-oauth must produce an ApiKey row");
    view.selected_index = api_key_idx;

    // @step When I read the footer hint
    let hint = view.footer_hint();

    // @step Then the hint contains "Enter:"
    assert!(
        hint.contains("Enter:"),
        "hint must contain 'Enter:': {hint:?}"
    );
    // @step And the hint contains "d: delete"
    assert!(
        hint.contains("d: delete"),
        "hint must contain 'd: delete': {hint:?}"
    );
    // @step And the hint does not contain "D Delete"
    assert!(
        !hint.contains("D Delete"),
        "hint must NOT contain 'D Delete': {hint:?}"
    );
    // @step And the hint does not contain "Esc Cancel"
    assert!(
        !hint.contains("Esc Cancel"),
        "hint must NOT contain 'Esc Cancel': {hint:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Footer updates in real time when the user navigates between
//           rows of different kinds
// ────────────────────────────────────────────────────────────────────────

#[test]
fn footer_updates_in_real_time_when_navigating_between_kinds() {
    // @step Given the ProviderSettingsView lists OpenAI with one profile and the
    // user starts on the OpenAI provider row
    let mut view = view_with(openai_with_profiles(&["prof1"]), &["openai"]);
    view.selected_index = 0;
    assert_eq!(
        view.footer_hint(),
        "Enter: expand · / filter · Tab: Switch to models · Esc: close",
        "precondition: provider-row hint at index 0"
    );

    // openai expanded with one profile produces:
    //   [0] Provider, [1] Profile { prof1 }, [2] AddProfile
    // (openai has is_oauth_provider=false and id==openai so ApiKey is skipped.)

    // @step When I move selection Down onto the ApiKey row
    // (openai uses Profile children — the "edit/delete" hint applies to both
    //  Profile and ApiKey identically; we use the Profile row here as the
    //  canonical "edit · delete" target.)
    view.selected_index = 1;
    // @step Then the footer hint equals "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        view.footer_hint(),
        "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close",
        "after Down once → Profile row"
    );

    // @step When I move selection Down onto the AddProfile row
    view.selected_index = 2;
    // @step Then the footer hint equals "Enter: create · / filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        view.footer_hint(),
        "Enter: create · / filter · Tab: Switch to models · Esc: close",
        "after Down twice → AddProfile row"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Footer hint is rendered into a single bottom row via
//           render_footer_hint
// ────────────────────────────────────────────────────────────────────────

#[test]
fn footer_hint_renders_into_bottom_buffer_row() {
    // @step Given the ProviderSettingsView has a Provider nav-item selected
    let mut view = view_with(openai_with_profiles(&[]), &[]);
    view.selected_index = 0;

    // @step When I render the view into a 80x10 buffer
    let area = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 10,
    };
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the bottom buffer row reads "Enter: expand · / filter · Tab: Switch to models · Esc: close" left-aligned
    let expected = "Enter: expand · / filter · Tab: Switch to models · Esc: close";
    let mut row = String::new();
    for x in 0..area.width {
        row.push_str(buf[(x, area.height - 1)].symbol());
    }
    // The row is padded with spaces to area.width; trim trailing spaces
    // before comparing, but the prefix must match exactly.
    assert!(
        row.starts_with(expected),
        "bottom row prefix mismatch.\n  expected: {expected:?}\n  actual:   {row:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: footer_hint_for(None) returns FOOTER_COMMON verbatim
// ────────────────────────────────────────────────────────────────────────

#[test]
fn footer_hint_for_none_returns_footer_common_verbatim() {
    // @step Given the canonical FOOTER_COMMON string "/ filter · Tab: Switch to models · Esc: close"
    assert_eq!(
        FOOTER_COMMON, EXPECTED_FOOTER_COMMON,
        "FOOTER_COMMON constant must match TS literal"
    );

    // @step When I call footer_hint_for with None
    let result = footer_hint_for(None);

    // @step Then the returned string equals FOOTER_COMMON exactly
    assert_eq!(result, FOOTER_COMMON, "footer_hint_for(None) mismatch");
}

// ────────────────────────────────────────────────────────────────────────
// Pure dispatch tests — exercise footer_hint_for for every RowKind in
// isolation (no view, no NavItem coupling).
// ────────────────────────────────────────────────────────────────────────

#[test]
fn footer_hint_for_pure_dispatch_provider() {
    assert_eq!(
        footer_hint_for(Some(RowKind::Provider { expanded: false })),
        "Enter: expand · / filter · Tab: Switch to models · Esc: close"
    );
    // Expansion state must not change the hint string.
    assert_eq!(
        footer_hint_for(Some(RowKind::Provider { expanded: true })),
        "Enter: expand · / filter · Tab: Switch to models · Esc: close"
    );
}

#[test]
fn footer_hint_for_pure_dispatch_api_key() {
    assert_eq!(
        footer_hint_for(Some(RowKind::ApiKey)),
        "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    );
}

#[test]
fn footer_hint_for_pure_dispatch_profile() {
    assert_eq!(
        footer_hint_for(Some(RowKind::Profile)),
        "Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close"
    );
}

#[test]
fn footer_hint_for_pure_dispatch_add_profile() {
    assert_eq!(
        footer_hint_for(Some(RowKind::AddProfile)),
        "Enter: create · / filter · Tab: Switch to models · Esc: close"
    );
}

#[test]
fn footer_hint_for_pure_dispatch_oauth_login() {
    assert_eq!(
        footer_hint_for(Some(RowKind::OauthLogin)),
        "Enter: start login · / filter · Tab: Switch to models · Esc: close"
    );
}

#[test]
fn footer_hint_for_pure_dispatch_oauth_status() {
    assert_eq!(
        footer_hint_for(Some(RowKind::OauthStatus)),
        "Enter: logout · / filter · Tab: Switch to models · Esc: close"
    );
}
