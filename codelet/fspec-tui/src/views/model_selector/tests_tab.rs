//! PROV-107 — RPC-345 Tab-returns-to-Provider-Settings tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: Tab in non-filter mode emits SwitchToProviders
#[test]
fn tab_in_non_filter_mode_emits_switch_to_providers() {
    // @step Given the model selector has loaded providers and is not in filter mode
    let mut v = loaded_view();
    assert!(!v.filter_mode);

    // @step When I press the Tab key
    let out = v.handle_key(key(KeyCode::Tab));

    // @step Then the model selector returns a SwitchToProviders event
    assert!(matches!(out, ModelSelectorEvent::SwitchToProviders));
}

/// Scenario: Tab while filtering does not navigate away
#[test]
fn tab_while_filtering_does_not_navigate_away() {
    // @step Given the model selector is in filter mode after I pressed "/"
    let mut v = loaded_view();
    v.handle_key(key(KeyCode::Char('/')));
    assert!(v.filter_mode);

    // @step When I press the Tab key
    let out = v.handle_key(key(KeyCode::Tab));

    // @step Then the model selector stays in filter mode
    assert!(v.filter_mode);
    // @step And no SwitchToProviders event is returned
    assert!(!matches!(out, ModelSelectorEvent::SwitchToProviders));
}

/// Scenario: Esc still closes the selector after the Tab arm is added
#[test]
fn esc_still_closes_after_tab_arm_added() {
    // @step Given the model selector has loaded providers and is not in filter mode
    let mut v = loaded_view();
    assert!(!v.filter_mode);

    // @step When I press the Esc key
    let out = v.handle_key(key(KeyCode::Esc));

    // @step Then the model selector returns a Close event
    assert!(matches!(out, ModelSelectorEvent::Close));
}

/// Scenario: SwitchToProviders switches the active view to Provider Settings
#[test]
fn switch_to_providers_flips_navigator_to_provider_settings() {
    use crate::components::Action;
    use crate::theme::Theme;
    use crate::views::navigator::{Navigator, ViewMode};
    use crossterm::event::{Event, KeyEventKind, KeyEventState};
    use std::sync::Arc;
    use tokio::sync::mpsc::unbounded_channel;

    // @step Given the Navigator is showing the model selector with providers loaded
    let (tx, mut rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.active_view = ViewMode::ModelSelector;

    // @step When I press the Tab key in the model selector
    let tab = Event::Key(KeyEvent {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let result = nav.handle_model_selector_event(&tab);
    assert!(result.is_consumed());

    // @step Then the Navigator sends the OpenProviderSettingsView action
    let action = rx.try_recv().expect("an action should be queued");
    assert!(matches!(action, Action::OpenProviderSettingsView));

    // @step And the active view becomes Provider Settings
    nav.apply_action(&action);
    assert_eq!(nav.active_view, ViewMode::ProviderSettings);
}
