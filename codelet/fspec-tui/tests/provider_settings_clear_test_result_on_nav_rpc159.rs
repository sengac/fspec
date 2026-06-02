//! RPC-159 — testResult clears on Up/Down arrow navigation in list mode.
//!
//! Feature: spec/features/rpc159-provider-settings-test-result-clears-on-nav.feature
//!
//! Mirrors the TS contract in `src/tui/inputHandlers/listModeHandler.ts`:
//! navigating up/down clears the inline test_result so the decoration
//! does not persist visually on a different focused row. Other keys
//! (Enter, Tab, '/', boundary arrows, arrows under filter_mode) MUST
//! preserve test_result.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::nav_item::ProviderDisplayInfo;
use codelet_fspec_tui::views::provider_settings::{
    ProviderSettingsEvent, ProviderSettingsView, ProviderTestStatus,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        masked_key: None,
        source: None,
    }
}

fn display_info(id: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: id.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
    }
}

fn three_providers() -> Vec<ProviderCredentialInfo> {
    vec![pinfo("openai"), pinfo("anthropic"), pinfo("groq")]
}

fn three_display_infos() -> Vec<ProviderDisplayInfo> {
    vec![
        display_info("openai"),
        display_info("anthropic"),
        display_info("groq"),
    ]
}

/// Build a list-mode view backed by the *legacy* `ProviderCredentialInfo`
/// list — arrow navigation goes through `visible_providers()` which uses
/// this list, so navigation-only tests can stay on this simpler API.
fn view_with_three() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(three_providers());
    v.set_visible_rows(5);
    v
}

/// Build a list-mode view backed by `display_providers` so `nav_items` is
/// populated. Required for tests that exercise the Enter → toggle_expansion
/// path (which routes through `focused_nav_item()`).
fn view_with_three_nav_items() -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_provider_display_infos(three_display_infos());
    v.set_visible_rows(5);
    v
}

fn last_visible_index(view: &ProviderSettingsView) -> usize {
    view.visible_provider_ids().len().saturating_sub(1)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down arrow that moves focus clears the inline test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_that_moves_focus_clears_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And selected_index is 1
    view.selected_index = 1;
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Down arrow key is dispatched to handle_list_key
    let out = view.handle_key(key(KeyCode::Down));

    // @step Then selected_index is 2
    assert_eq!(view.selected_index, 2);
    // @step And test_result is None
    assert!(view.test_result.is_none());
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Up arrow that moves focus clears the inline test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_that_moves_focus_clears_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And selected_index is 2
    view.selected_index = 2;
    // @step And test_result is set to Some(provider_id="openai", status=Err{message="boom"})
    view.set_test_result(
        "openai",
        ProviderTestStatus::Err {
            message: "boom".to_string(),
        },
    );

    // @step When the Up arrow key is dispatched to handle_list_key
    let out = view.handle_key(key(KeyCode::Up));

    // @step Then selected_index is 1
    assert_eq!(view.selected_index, 1);
    // @step And test_result is None
    assert!(view.test_result.is_none());
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Up arrow at index 0 does not clear test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_at_index_zero_does_not_clear_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And selected_index is 0
    view.selected_index = 0;
    // @step And test_result is set to Some(provider_id="openai", status=Testing)
    view.set_test_result("openai", ProviderTestStatus::Testing);

    // @step When the Up arrow key is dispatched to handle_list_key
    let out = view.handle_key(key(KeyCode::Up));

    // @step Then selected_index is still 0
    assert_eq!(view.selected_index, 0);
    // @step And test_result is still Some(provider_id="openai", status=Testing)
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Testing);
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down arrow at last visible row does not clear test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_at_last_visible_row_does_not_clear_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And selected_index is at the last visible nav-item index
    let last = last_visible_index(&view);
    view.selected_index = last;
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Down arrow key is dispatched to handle_list_key
    let out = view.handle_key(key(KeyCode::Down));

    // @step Then selected_index is unchanged
    assert_eq!(view.selected_index, last);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a Provider row toggles expansion and preserves test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_provider_row_preserves_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three_nav_items();
    // @step And the focused nav item is a Provider row
    view.selected_index = 0;
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });
    let expanded_before = view.expanded.contains("openai");

    // @step When the Enter key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Enter));

    // @step Then the focused provider's expansion is toggled
    let expanded_after = view.expanded.contains("openai");
    assert_ne!(expanded_before, expanded_after);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Tab in list mode emits SwitchToModels and preserves test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn tab_in_list_mode_emits_switch_to_models_and_preserves_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Tab key is dispatched to handle_list_key
    let out = view.handle_key(key(KeyCode::Tab));

    // @step Then the returned event is SwitchToModels
    assert!(matches!(out, ProviderSettingsEvent::SwitchToModels));
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Slash activates filter mode and preserves test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn slash_activates_filter_mode_and_preserves_test_result() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the '/' key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Char('/')));

    // @step Then filter_mode is true
    assert!(view.filter_mode);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Up arrow while filter_mode is true does not enter list-mode clear path
// ────────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_in_filter_mode_does_not_clear_test_result() {
    // @step Given a ProviderSettingsView with three providers
    let mut view = view_with_three();
    // @step And filter_mode is true
    view.filter_mode = true;
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Up arrow key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Up));

    // @step Then the call is routed through handle_filter_key
    // (verified by: filter_mode remains true, test_result untouched — the
    //  list-mode clear path would have set test_result to None.)
    assert!(view.filter_mode);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down arrow that moves focus with test_result already None remains None
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_with_none_test_result_remains_none() {
    // @step Given a ProviderSettingsView in List mode with three providers
    let mut view = view_with_three();
    // @step And selected_index is 1
    view.selected_index = 1;
    // @step And test_result is None
    assert!(view.test_result.is_none());

    // @step When the Down arrow key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Down));

    // @step Then selected_index is 2
    assert_eq!(view.selected_index, 2);
    // @step And test_result is still None
    assert!(view.test_result.is_none());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Up or Down arrow with zero visible providers does not clear test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn arrows_with_zero_visible_providers_preserve_test_result() {
    // @step Given a ProviderSettingsView in List mode with zero visible providers
    let mut view = ProviderSettingsView::new();
    view.set_providers(Vec::new());
    view.set_visible_rows(5);
    // @step And selected_index is 0
    assert_eq!(view.selected_index, 0);
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Down arrow key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Down));
    // @step Then selected_index is still 0
    assert_eq!(view.selected_index, 0);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Up arrow key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Up));
    // @step Then selected_index is still 0
    assert_eq!(view.selected_index, 0);
    // @step And test_result is still Some(provider_id="openai", status=Ok{latency_ms=42})
    let tr = view.test_result.as_ref().expect("test_result preserved");
    assert_eq!(tr.provider_id, "openai");
    assert_eq!(tr.status, ProviderTestStatus::Ok { latency_ms: 42 });
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down arrow that scrolls and moves focus adjusts scroll AND clears
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_that_scrolls_also_clears_test_result() {
    // @step Given a ProviderSettingsView in List mode with many providers requiring scrolling
    let mut view = view_with_three();
    // @step And selected_index is 1
    view.selected_index = 1;
    // @step And scroll_offset is 0
    view.scroll_offset = 0;
    // @step And visible_rows is 2
    view.set_visible_rows(2);
    // @step And test_result is set to Some(provider_id="openai", status=Ok{latency_ms=42})
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 42 });

    // @step When the Down arrow key is dispatched to handle_list_key
    let _ = view.handle_key(key(KeyCode::Down));

    // @step Then selected_index is 2
    assert_eq!(view.selected_index, 2);
    // @step And scroll_offset has advanced
    assert!(view.scroll_offset > 0, "scroll_offset should advance");
    // @step And test_result is None
    assert!(view.test_result.is_none());
}
