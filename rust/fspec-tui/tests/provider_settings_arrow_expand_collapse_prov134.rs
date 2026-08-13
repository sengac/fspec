//! PROV-134 — Left/Right arrow expand/collapse in the /provider list view.
//!
//! Feature: spec/features/provider-settings-arrow-expand-collapse.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy: build a `ProviderSettingsView` in List mode via
//! `set_provider_display_infos`, seed/clear expansion with `toggle_expansion`,
//! park the cursor on a specific row via `selected_index`, then drive a single
//! Left/Right `KeyEvent` through the public `handle_key` entry point and assert
//! on `view.expanded` membership plus `view.selected_index`.
//!
//! RED PHASE: `handle_list_key` currently has NO Left/Right arms, so these keys
//! are swallowed as no-ops. The expand/collapse assertions therefore FAIL until
//! the fix lands. No production code is touched by this file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::nav_item::{NavItemKind, ProviderDisplayInfo};
use codelet_fspec_tui::views::ProviderSettingsView;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

/// An OpenAI provider that yields child rows on expansion — openai with one
/// profile produces `[provider(0), profile(1), add-profile(2)]` when expanded.
fn openai_with_profile() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "openai".to_string(),
        name: "OpenAI API".to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: vec!["qwen".to_string()],
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

/// The nav-tree index of the openai PROVIDER header row.
fn openai_header_index(view: &ProviderSettingsView) -> usize {
    view.nav_items
        .iter()
        .position(|item| {
            item.provider_id == "openai" && matches!(item.kind, NavItemKind::Provider { .. })
        })
        .expect("openai provider header row should exist in nav_items")
}

/// The nav-tree index of the first NON-provider (child) row.
fn first_child_index(view: &ProviderSettingsView) -> usize {
    view.nav_items
        .iter()
        .position(|item| !matches!(item.kind, NavItemKind::Provider { .. }))
        .expect("an expanded openai provider should have at least one child row")
}

fn press(view: &mut ProviderSettingsView, code: KeyCode) {
    view.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Right arrow expands a collapsed provider header
// ════════════════════════════════════════════════════════════════════════
#[test]
fn right_arrow_expands_collapsed_provider_header() {
    // @step Given the provider list is showing with the OpenAI provider collapsed
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_with_profile()]);
    assert!(
        !view.expanded.contains("openai"),
        "precondition: openai should start collapsed"
    );

    // @step Given the cursor is focused on the OpenAI provider header row
    let header = openai_header_index(&view);
    view.selected_index = header;

    // @step When I press the Right arrow key
    press(&mut view, KeyCode::Right);

    // @step Then the OpenAI provider becomes expanded
    assert!(
        view.expanded.contains("openai"),
        "Right arrow on a collapsed header should expand openai"
    );

    // @step Then the cursor remains on the OpenAI provider header row
    assert_eq!(
        view.selected_index, header,
        "cursor should remain on the openai header row"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Right arrow on an already-expanded provider header does nothing
// ════════════════════════════════════════════════════════════════════════
#[test]
fn right_arrow_on_expanded_header_does_nothing() {
    // @step Given the provider list is showing with the OpenAI provider expanded
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_with_profile()]);
    view.toggle_expansion("openai");
    assert!(
        view.expanded.contains("openai"),
        "precondition: openai should start expanded"
    );

    // @step Given the cursor is focused on the OpenAI provider header row
    let header = openai_header_index(&view);
    view.selected_index = header;

    // @step When I press the Right arrow key
    press(&mut view, KeyCode::Right);

    // @step Then the OpenAI provider remains expanded
    assert!(
        view.expanded.contains("openai"),
        "Right arrow on an already-expanded header should remain expanded"
    );

    // @step Then the cursor remains on the OpenAI provider header row
    assert_eq!(
        view.selected_index, header,
        "cursor should remain on the openai header row"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Left arrow collapses an expanded provider header
// ════════════════════════════════════════════════════════════════════════
#[test]
fn left_arrow_collapses_expanded_provider_header() {
    // @step Given the provider list is showing with the OpenAI provider expanded
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_with_profile()]);
    view.toggle_expansion("openai");
    assert!(
        view.expanded.contains("openai"),
        "precondition: openai should start expanded"
    );

    // @step Given the cursor is focused on the OpenAI provider header row
    let header = openai_header_index(&view);
    view.selected_index = header;

    // @step When I press the Left arrow key
    press(&mut view, KeyCode::Left);

    // @step Then the OpenAI provider becomes collapsed
    assert!(
        !view.expanded.contains("openai"),
        "Left arrow on an expanded header should collapse openai"
    );

    // @step Then the cursor remains on the OpenAI provider header row
    assert_eq!(
        view.selected_index, header,
        "cursor should remain on the openai header row"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Left arrow on an already-collapsed provider header does nothing
// ════════════════════════════════════════════════════════════════════════
#[test]
fn left_arrow_on_collapsed_header_does_nothing() {
    // @step Given the provider list is showing with the OpenAI provider collapsed
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_with_profile()]);
    assert!(
        !view.expanded.contains("openai"),
        "precondition: openai should start collapsed"
    );

    // @step Given the cursor is focused on the OpenAI provider header row
    let header = openai_header_index(&view);
    view.selected_index = header;

    // @step When I press the Left arrow key
    press(&mut view, KeyCode::Left);

    // @step Then the OpenAI provider remains collapsed
    assert!(
        !view.expanded.contains("openai"),
        "Left arrow on an already-collapsed header should remain collapsed"
    );

    // @step Then the cursor remains on the OpenAI provider header row
    assert_eq!(
        view.selected_index, header,
        "cursor should remain on the openai header row"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Left arrow on a child row does nothing
// ════════════════════════════════════════════════════════════════════════
#[test]
fn left_arrow_on_child_row_does_nothing() {
    // @step Given the provider list is showing with the OpenAI provider expanded
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_with_profile()]);
    view.toggle_expansion("openai");
    assert!(
        view.expanded.contains("openai"),
        "precondition: openai should start expanded"
    );

    // @step Given the cursor is focused on a child row under the OpenAI provider
    let child = first_child_index(&view);
    view.selected_index = child;

    // @step When I press the Left arrow key
    press(&mut view, KeyCode::Left);

    // @step Then the OpenAI provider remains expanded
    assert!(
        view.expanded.contains("openai"),
        "Left arrow on a child row must NOT collapse the parent header"
    );

    // @step Then the cursor remains on the same child row
    assert_eq!(
        view.selected_index, child,
        "cursor should remain on the same child row"
    );
}
