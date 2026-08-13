//! RPC-337 — Navigator + dispatch wiring for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Drives the Navigator's ViewMode flips for the model selector:
//!   * `Action::OpenModelSelectorView` → `ViewMode::ModelSelector`
//!   * `Action::CloseModelSelectorView` → `ViewMode::Agent`
//!   * `Action::ModelSelected(..)` (commit) → returns to `ViewMode::Agent`
//!   * ProviderSettings `Tab` (SwitchToModels) → `ViewMode::ModelSelector`
//!
//! These run the Navigator in isolation (no App / backend), asserting
//! the `active_view` transitions + the action emitted onto the bus.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::store::BoardStore;
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::{Navigator, ViewMode};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

fn fresh() -> (Navigator, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    (Navigator::new(Arc::new(Theme::default()), tx), rx)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Scenario: Open the model selector full-screen via the slash command
#[test]
fn open_model_selector_view_flips_active_view() {
    // @step Given I am in the Agent view
    let (mut nav, _rx) = fresh();
    nav.apply_action(&Action::EnterWorkUnit("AUTH-001".to_string()));
    assert_eq!(nav.active_view, ViewMode::Agent);

    // @step When I run the "/model" slash command
    nav.apply_action(&Action::OpenModelSelectorView);

    // @step Then the model selector replaces the screen as a full-screen view
    assert_eq!(nav.active_view, ViewMode::ModelSelector);
}

/// Scenario: Close the model selector with Esc returns to Agent
#[test]
fn close_model_selector_view_returns_to_agent() {
    // @step Given I am in the model selector mode-view
    let (mut nav, _rx) = fresh();
    nav.apply_action(&Action::OpenModelSelectorView);
    assert_eq!(nav.active_view, ViewMode::ModelSelector);

    // @step When I press Esc
    nav.apply_action(&Action::CloseModelSelectorView);

    // @step Then the model selector closes
    // @step And I am returned to the Agent view
    assert_eq!(nav.active_view, ViewMode::Agent);
}

/// Scenario: Selecting a model with an active session commits the choice
#[test]
fn model_selected_returns_to_agent_view() {
    // @step Given the model selector is open with an active session
    let (mut nav, _rx) = fresh();
    nav.apply_action(&Action::OpenModelSelectorView);
    assert_eq!(nav.active_view, ViewMode::ModelSelector);

    // @step When I press Enter
    // (the view emits ModelSelected; the Navigator commits + closes)
    nav.apply_action(&Action::ModelSelected(
        Some(codelet_rpc_types::SessionId::new("s-1")),
        "openai".to_string(),
        "gpt-4o".to_string(),
    ));

    // @step Then a model selection is emitted for the current session, provider and model
    // @step And the model selector view closes
    assert_eq!(nav.active_view, ViewMode::Agent);
}

/// Scenario: Switch from Provider Settings to the model selector with Tab
#[test]
fn tab_in_provider_settings_emits_open_model_selector() {
    // @step Given I am in the Provider Settings view in list mode
    let (mut nav, mut rx) = fresh();
    nav.apply_action(&Action::OpenProviderSettingsView);
    assert_eq!(nav.active_view, ViewMode::ProviderSettings);
    let board = BoardStore::default();

    // @step When I press Tab
    let _ = nav.handle_event(&key(KeyCode::Tab), &board);

    // @step Then the Navigator flips to the model selector mode-view full-screen
    // The SwitchToModels outcome is translated to Action::OpenModelSelectorView
    // on the bus; applying it flips the view (App applies dispatched actions).
    let action = rx.try_recv().expect("expected an action on the bus");
    assert!(
        matches!(action, Action::OpenModelSelectorView),
        "Tab in ProviderSettings must emit OpenModelSelectorView, got {action:?}"
    );
    nav.apply_action(&action);
    assert_eq!(nav.active_view, ViewMode::ModelSelector);
}
