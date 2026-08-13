//! PROV-103 — Up/Down and Enter navigation across the full flat nav tree.
//!
//! Feature: spec/features/provider-settings-nav-tree-navigation.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map 1:1 to the Gherkin scenarios. Tests are written
//! test-first per ACDD: they FAIL against the current `provider_settings/mod.rs`
//! (whose `move_clamped`/`adjust_scroll` bound navigation by
//! `visible_providers().len()` — top-level providers only) and PASS once the
//! fix bounds them by `nav_items.len()` when nav_items is populated.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::{
    DetailSub, NavItemKind, OAuthMethod, ProviderDisplayInfo, ProviderSettingsMode,
    ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// openai-shaped profile provider with N profiles (yields profile + add-profile
/// child rows when expanded).
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

/// anthropic-shaped OAuth provider — yields oauth-login + api-key child rows
/// when expanded (no oauth-status because has_oauth_tokens=false).
fn anthropic() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured: false,
        credential_type: "oauth".to_string(),
        model_count: 8,
        has_oauth_tokens: false,
        is_oauth_provider: true,
        requires_api_key: true,
        env_var: Some("ANTHROPIC_API_KEY".to_string()),
        profiles: Vec::new(),
        oauth_login_methods: vec![
            (OAuthMethod::Browser, "Sign in with browser".to_string()),
            (OAuthMethod::Headless, "Sign in with code".to_string()),
        ],
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

fn pinfo(id: &str, ctype: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: false,
        credential_type: ctype.to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

/// Index of the openai provider header row in nav_items.
fn provider_row_index(view: &ProviderSettingsView, id: &str) -> usize {
    view.nav_items
        .iter()
        .position(|n| n.provider_id == id && matches!(n.kind, NavItemKind::Provider { .. }))
        .expect("provider row must exist in nav_items")
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down from an expanded provider lands on its first child row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_from_expanded_provider_lands_on_first_child_row() {
    // @step Given a provider settings view with an expanded openai provider that has profiles
    let mut view = ProviderSettingsView::new();
    view.set_visible_rows(18);
    view.set_provider_display_infos(vec![openai_with_profiles(&["prof1", "prof2"])]);
    view.toggle_expansion("openai");

    // @step And the nav tree has more rows than the single top-level provider
    assert!(
        view.nav_items.len() > 1,
        "expanded openai must produce child rows in nav_items"
    );

    // @step And the cursor is on the openai provider row
    let provider_idx = provider_row_index(&view, "openai");
    view.selected_index = provider_idx;

    // @step When the user presses Down
    view.handle_key(key(KeyCode::Down));

    // @step Then the cursor moves to the first child row beneath the provider
    assert_eq!(
        view.selected_index,
        provider_idx + 1,
        "Down must move onto the first child row, not be trapped at the provider count"
    );

    // @step And the focused nav item belongs to the openai provider child block
    let focused = view
        .focused_nav_item()
        .expect("focused nav item must exist after navigating onto a child row");
    assert_eq!(focused.provider_id, "openai");
    assert!(
        !matches!(focused.kind, NavItemKind::Provider { .. }),
        "the landed row must be a child row, not the provider header"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Repeated Down reaches every child row and scrolls the
//           highlighted row into view
// ────────────────────────────────────────────────────────────────────────

#[test]
fn repeated_down_reaches_child_rows_beyond_viewport_and_scrolls() {
    // @step Given a provider settings view with an expanded provider in a nav tree taller than the viewport
    let mut view = ProviderSettingsView::new();
    // Many profiles so the expanded tree is taller than the viewport.
    let profiles: Vec<String> = (0..20).map(|i| format!("prof{i}")).collect();
    let profile_refs: Vec<&str> = profiles.iter().map(String::as_str).collect();
    view.set_provider_display_infos(vec![openai_with_profiles(&profile_refs)]);
    view.toggle_expansion("openai");

    // @step And the viewport shows fewer rows than the nav tree contains
    let viewport: usize = 5;
    view.set_visible_rows(viewport);
    assert!(
        view.nav_items.len() > viewport,
        "nav tree must be taller than the viewport to exercise scrolling"
    );
    view.selected_index = 0;
    view.scroll_offset = 0;

    // @step When the user presses Down enough times to move past the viewport height
    for _ in 0..(viewport + 2) {
        view.handle_key(key(KeyCode::Down));
    }

    // @step Then the cursor reaches a child row beyond the first viewport
    assert!(
        view.selected_index >= viewport,
        "cursor must move beyond the first viewport (got index {})",
        view.selected_index
    );
    assert!(
        view.selected_index < view.nav_items.len(),
        "cursor must stay within nav_items bounds"
    );

    // @step And the scroll offset advances past zero so the highlighted row stays visible
    assert!(
        view.scroll_offset > 0,
        "scroll_offset must advance past 0 to keep the highlighted row visible"
    );
    assert!(
        view.selected_index >= view.scroll_offset
            && view.selected_index < view.scroll_offset + viewport,
        "highlighted row must be inside the visible window [{}, {})",
        view.scroll_offset,
        view.scroll_offset + viewport
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on an api-key child row opens the edit-api-key detail
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_api_key_child_row_opens_edit_api_key_detail() {
    // @step Given a provider settings view with an expanded provider exposing an api-key child row
    let mut view = ProviderSettingsView::new();
    view.set_visible_rows(18);
    view.set_provider_display_infos(vec![anthropic()]);
    view.toggle_expansion("anthropic");

    // @step And the cursor is on the api-key child row
    let api_key_idx = view
        .nav_items
        .iter()
        .position(|n| matches!(n.kind, NavItemKind::ApiKey))
        .expect("expanded anthropic must yield an ApiKey child row");
    view.selected_index = api_key_idx;

    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the view transitions to the edit-api-key detail for that provider
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "anthropic");
            assert!(
                matches!(sub, DetailSub::EditApiKey { .. }),
                "Enter on an api-key child row must open EditApiKey, got {sub:?}"
            );
        }
        other => panic!("expected Detail mode after Enter, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Navigation is clamped with no wrap at the tree boundaries
// ────────────────────────────────────────────────────────────────────────

#[test]
fn navigation_is_clamped_with_no_wrap_at_tree_boundaries() {
    // @step Given a provider settings view with an expanded provider producing several nav rows
    let mut view = ProviderSettingsView::new();
    view.set_visible_rows(18);
    view.set_provider_display_infos(vec![openai_with_profiles(&["prof1", "prof2"])]);
    view.toggle_expansion("openai");
    let last = view.nav_items.len() - 1;
    assert!(last > 0, "need several nav rows for boundary clamping");

    // @step And the cursor is on the last nav row
    view.selected_index = last;

    // @step When the user presses Down
    view.handle_key(key(KeyCode::Down));

    // @step Then the cursor stays on the last nav row
    assert_eq!(
        view.selected_index, last,
        "Down at the last row must be a clamped no-op (no wrap, no overflow)"
    );

    // @step When the cursor is moved to the first nav row
    view.selected_index = 0;

    // @step And the user presses Up
    view.handle_key(key(KeyCode::Up));

    // @step Then the cursor stays on the first nav row
    assert_eq!(
        view.selected_index, 0,
        "Up at the first row must be a clamped no-op (no wrap)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Empty nav tree falls back to provider-count bounds without
//           silent selection
// ────────────────────────────────────────────────────────────────────────

#[test]
fn empty_nav_tree_falls_back_to_provider_count_bounds_without_silent_selection() {
    // @step Given a provider settings view populated only via the legacy provider list with two providers
    let mut view = ProviderSettingsView::new();
    view.set_visible_rows(18);
    view.set_providers(vec![
        pinfo("openai", "api_key"),
        pinfo("anthropic", "oauth"),
    ]);

    // @step And the nav items list is empty
    assert!(
        view.nav_items.is_empty(),
        "legacy set_providers must not populate nav_items"
    );

    // @step And the cursor is on the first provider
    view.selected_index = 0;

    // @step When the user presses Down
    view.handle_key(key(KeyCode::Down));

    // @step Then the cursor moves to the second provider
    assert_eq!(
        view.selected_index, 1,
        "legacy fallback must still allow stepping within visible_providers"
    );

    // @step When the user presses Down again
    view.handle_key(key(KeyCode::Down));

    // @step Then the cursor stays on the second provider and no selection is silently snapped
    assert_eq!(
        view.selected_index, 1,
        "Down past the last legacy provider must clamp (no overflow, no silent snap)"
    );
}
