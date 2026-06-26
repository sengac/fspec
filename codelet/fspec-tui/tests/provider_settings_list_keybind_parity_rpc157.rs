//! RPC-157 — ProviderSettingsView list-mode keybind parity with TS.
//!
//! Feature: spec/features/rpc157-provider-settings-list-mode-keybind-parity.feature
//!
//! This test file validates the acceptance criteria for RPC-157: drop
//! the four Rust-only list-mode keybind groups (arrow wrap-around,
//! PageUp/PageDown, Home/End, list-mode r/R) so the Rust ratatui
//! ProviderSettingsView matches the TS Ink reference (listModeHandler.ts).
//!
//! Tests are written test-first per ACDD: they FAIL against the current
//! list.rs (which still wraps arrows + binds PageUp/PageDown/Home/End)
//! and PASS once RPC-157 implementation lands.
//!
//! Scenarios map 1:1 to Gherkin scenarios in the feature file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
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

fn seventeen_providers() -> Vec<ProviderCredentialInfo> {
    // Canonical 17 provider IDs, mirroring TS SUPPORTED_PROVIDERS (RPC-107).
    // The actual canonical catalog lands in RPC-107; we use placeholder
    // IDs here because RPC-157 is about keybind dispatch shape, not catalog content.
    let ids = [
        "openai",
        "anthropic",
        "cohere",
        "gemini",
        "mistral",
        "xai",
        "together",
        "huggingface",
        "openrouter",
        "groq",
        "deepseek",
        "moonshot",
        "galadriel",
        "azure",
        "zai",
        "codex",
        "github-copilot",
    ];
    ids.iter()
        .map(|id| pinfo(id, "api_key", false, 0))
        .collect()
}

fn list_view_with(providers: Vec<ProviderCredentialInfo>) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(providers);
    v
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Up arrow at the first nav item is a clamped no-op (no wrap-around)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_at_first_nav_item_is_clamped_no_op() {
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    // @step And the cursor is on the first nav item (OpenAI API, index 0)
    assert_eq!(view.selected_index, 0);
    // @step When I press the Up arrow key
    let out = view.handle_key(key(KeyCode::Up));
    // @step Then the cursor remains on the first nav item (index 0)
    assert_eq!(view.selected_index, 0);
    // @step And the cursor does NOT wrap to the last nav item (index 16)
    assert_ne!(view.selected_index, 16);
    // @step And no other view state changes
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    assert!(view.delete_confirm.is_none());
    assert!(view.filter.is_empty());
    assert!(!view.filter_mode);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Down arrow at the last nav item is a clamped no-op (no wrap-around)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn down_arrow_at_last_nav_item_is_clamped_no_op() {
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    view.set_visible_rows(18);
    // @step And the cursor is on the last nav item (GitHub Copilot, index 16)
    view.selected_index = 16;
    // (scroll adjusted on next render)
    assert_eq!(view.selected_index, 16);
    // @step When I press the Down arrow key
    let out = view.handle_key(key(KeyCode::Down));
    // @step Then the cursor remains on the last nav item (index 16)
    assert_eq!(view.selected_index, 16);
    // @step And the cursor does NOT wrap to the first nav item (index 0)
    assert_ne!(view.selected_index, 0);
    // @step And no other view state changes
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    assert!(view.delete_confirm.is_none());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: PageDown pages the list (RPC-353 SUPERSEDES the RPC-157 no-op)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pagedown_is_silently_ignored_in_list_mode() {
    // RPC-353 supersedes RPC-157 for paging keys: /provider now pages by a
    // viewport on PageDown as a deliberate enhancement beyond TS parity.
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    view.set_visible_rows(18);
    // @step And the cursor is on the Mistral AI nav item (index 5)
    view.selected_index = 5;
    // @step When I press the PageDown key
    let out = view.handle_key(key(KeyCode::PageDown));
    // @step Then the cursor advances by one page (clamped to the last item)
    assert_eq!(view.selected_index, 16);
    // @step And the key is consumed without emitting an Action
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    if let ProviderSettingsEvent::Emit(_) = out {
        panic!("PageDown must not emit an Action");
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: PageUp pages the list (RPC-353 SUPERSEDES the RPC-157 no-op)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pageup_is_silently_ignored_in_list_mode() {
    // RPC-353 supersedes RPC-157: /provider pages up by a viewport on PageUp.
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    view.set_visible_rows(18);
    // @step And the cursor is on index 5
    view.selected_index = 5;
    // @step When I press the PageUp key
    let out = view.handle_key(key(KeyCode::PageUp));
    // @step Then the cursor retreats by one page (clamped to the first item)
    assert_eq!(view.selected_index, 0);
    // @step And the key is consumed without emitting an Action
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    if let ProviderSettingsEvent::Emit(_) = out {
        panic!("PageUp must not emit an Action");
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Home jumps to the first item (RPC-353 SUPERSEDES RPC-157 no-op)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn home_is_silently_ignored_in_list_mode() {
    // RPC-353 supersedes RPC-157: Home jumps to the first nav item.
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    view.set_visible_rows(18);
    // @step And the cursor is on index 8
    view.selected_index = 8;
    // @step When I press the Home key
    let out = view.handle_key(key(KeyCode::Home));
    // @step Then the cursor jumps to index 0
    assert_eq!(view.selected_index, 0);
    // @step And the key is consumed without emitting an Action
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    if let ProviderSettingsEvent::Emit(_) = out {
        panic!("Home must not emit an Action");
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: End jumps to the last item (RPC-353 SUPERSEDES RPC-157 no-op)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn end_is_silently_ignored_in_list_mode() {
    // RPC-353 supersedes RPC-157: End jumps to the last nav item.
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    view.set_visible_rows(18);
    // @step And the cursor is on index 8
    view.selected_index = 8;
    // @step When I press the End key
    let out = view.handle_key(key(KeyCode::End));
    // @step Then the cursor jumps to the last nav item (index 16)
    assert_eq!(view.selected_index, 16);
    // @step And the key is consumed without emitting an Action
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    if let ProviderSettingsEvent::Emit(_) = out {
        panic!("End must not emit an Action");
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Lowercase "r" is silently ignored in list mode (no RefreshProviderModels)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn lowercase_r_is_silently_ignored_in_list_mode() {
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    // @step And the cursor is on the OpenAI API provider row
    view.selected_index = 0;
    // @step When I press the "r" key
    let out = view.handle_key(key(KeyCode::Char('r')));
    // @step Then no Action::RefreshProviderModels is emitted
    if let ProviderSettingsEvent::Emit(action) = &out {
        if matches!(action, Action::RefreshProviderModels(_)) {
            panic!("'r' in list mode must not emit RefreshProviderModels");
        }
        panic!("'r' in list mode must not emit any Action, got {action:?}");
    }
    // @step And no backend RPC is fired
    // @step And no tracing log fires
    // @step And the view re-renders unchanged
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    assert_eq!(view.selected_index, 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Uppercase "R" is silently ignored in list mode (no RefreshProviderModels)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn uppercase_r_is_silently_ignored_in_list_mode() {
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());
    // @step And the cursor is on the OpenAI API provider row
    view.selected_index = 0;
    // @step When I press the "R" key (shifted)
    let out = view.handle_key(key(KeyCode::Char('R')));
    // @step Then no Action::RefreshProviderModels is emitted
    if let ProviderSettingsEvent::Emit(action) = &out {
        if matches!(action, Action::RefreshProviderModels(_)) {
            panic!("'R' in list mode must not emit RefreshProviderModels");
        }
        panic!("'R' in list mode must not emit any Action, got {action:?}");
    }
    // @step And no backend RPC is fired
    // @step And no tracing log fires
    // @step And the view re-renders unchanged
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    assert_eq!(view.selected_index, 0);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Preserved TS-canonical keybinds continue to behave per the existing contract
// ────────────────────────────────────────────────────────────────────────

#[test]
fn preserved_ts_canonical_keybinds_still_dispatch_their_actions() {
    // @step Given I have opened /provider with the 17 canonical providers loaded
    let mut view = list_view_with(seventeen_providers());

    // @step When I press each of the keys ↑, ↓, Enter, "/", Tab, Esc, and "d" in turn
    // We exercise each preserved keybind in a fresh-state way so that
    // earlier dispatches don't fight later ones.

    // ↓ — moves cursor forward (clamped)
    view.selected_index = 0;
    view.handle_key(key(KeyCode::Down));
    // @step Then each key continues to dispatch its TS-canonical action per RPC-054, RPC-103, RPC-106, RPC-160, and RPC-164
    assert_eq!(view.selected_index, 1, "↓ should advance cursor by 1");

    // ↑ — moves cursor backward (clamped)
    view.handle_key(key(KeyCode::Up));
    assert_eq!(view.selected_index, 0, "↑ should retreat cursor by 1");

    // "/" — enters filter mode
    view.filter_mode = false;
    view.handle_key(key(KeyCode::Char('/')));
    assert!(view.filter_mode, "'/' should enter filter mode");

    // Exit filter mode for the next assertions
    view.filter_mode = false;

    // Esc — emits Close when filter is empty
    let out = view.handle_key(key(KeyCode::Esc));
    assert!(
        matches!(out, ProviderSettingsEvent::Close),
        "Esc with empty filter should emit Close"
    );

    // Enter — transitions to Detail (RPC-054 contract; will become expansion under RPC-103)
    let mut view2 = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view2.handle_key(key(KeyCode::Enter));
    assert!(
        matches!(view2.mode, ProviderSettingsMode::Detail { .. }),
        "Enter on an api_key provider should transition to Detail"
    );

    // d — opens ConfirmDialog on a configured row
    let mut view3 = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view3.handle_key(key(KeyCode::Char('d')));
    assert!(
        view3.delete_confirm.is_some(),
        "'d' on a configured provider should open ConfirmDialog"
    );

    // @step And only the four Rust-only keybind groups are removed (r/R, arrow wrap-around, PageUp/Down, Home/End)
    // @step And no other behaviour regresses
    // (verified by the preceding assertions and by the sibling per-keybind tests above)
}
