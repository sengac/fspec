//! RPC-353 — Mouse wheel + Page/Home/End scroll for /provider and /model.
//!
//! Feature: spec/features/mode-view-scroll-input.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy: drive raw `Event`s through `Navigator::handle_event` (the live
//! routing entry point) and assert the per-view selection/scroll state
//! changes. Mouse events were previously dropped at the navigator; paging
//! keys were absent on /provider.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::store::BoardStore;
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{Navigator, ViewMode};
use codelet_rpc_types::{ModelEntry, ProviderCredentialInfo, ProviderInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseEvent, MouseEventKind,
};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn fresh() -> Navigator {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    Navigator::new(Arc::new(Theme::default()), tx)
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn wheel(kind: MouseEventKind) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: 5,
        row: 5,
        modifiers: KeyModifiers::NONE,
    })
}

fn pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 4,
        masked_key: None,
        source: None,
    }
}

fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 200_000,
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }
}

/// Navigator with /provider active and a 30-item nav tree, body height seeded.
fn provider_nav() -> Navigator {
    let mut nav = fresh();
    let infos: Vec<ProviderCredentialInfo> = (0..30).map(|i| pinfo(&format!("prov{i}"))).collect();
    nav.provider_settings
        .set_provider_display_infos(project_display_infos(&infos, &[]));
    nav.provider_settings.set_visible_rows(10);
    nav.active_view = ViewMode::ProviderSettings;
    nav
}

/// Navigator with /model active and a 30-model tree (expanded), height seeded.
fn model_nav() -> Navigator {
    let mut nav = fresh();
    let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let provider = ProviderInfo {
        key: "openai".to_string(),
        display_name: "openai".to_string(),
        models: refs.iter().map(|i| model(i)).collect(),
        profile_name: None,
        is_unreachable: false,
    };
    nav.model_selector.set_providers(vec![provider]);
    // Expand the provider so its 30 model rows are selectable, then render
    // once into a TestBackend so visible_rows is populated.
    nav.model_selector
        .handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 14)).expect("term");
    term.draw(|f| nav.model_selector.render(f.area(), f.buffer_mut()))
        .expect("draw");
    // Anchor a live selection (set_providers leaves has_selection=false when
    // there is no current model — a Down key activates it).
    nav.model_selector
        .handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    nav.active_view = ViewMode::ModelSelector;
    nav
}

fn board() -> BoardStore {
    BoardStore::default()
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Mouse wheel scrolls the provider list
// ────────────────────────────────────────────────────────────────────────

#[test]
fn mouse_wheel_scrolls_the_provider_list() {
    // @step Given the /provider view is active with a list longer than the viewport
    let mut nav = provider_nav();
    let b = board();
    let before = nav.provider_settings.selected_index;

    // @step When a ScrollDown mouse event is routed through the navigator
    let res = nav.handle_event(&wheel(MouseEventKind::ScrollDown), &b);

    // @step Then the provider selection moves downward
    assert!(res.is_consumed(), "wheel event must be consumed");
    assert!(
        nav.provider_settings.selected_index > before,
        "selection moved down: before={before} after={}",
        nav.provider_settings.selected_index
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Mouse wheel scrolls the model list
// ────────────────────────────────────────────────────────────────────────

#[test]
fn mouse_wheel_scrolls_the_model_list() {
    // @step Given the /model view is active with a list longer than the viewport
    let mut nav = model_nav();
    let b = board();
    let before = nav.model_selector.selected_index();

    // @step When a ScrollDown mouse event is routed through the navigator
    let res = nav.handle_event(&wheel(MouseEventKind::ScrollDown), &b);

    // @step Then the model selection moves downward
    assert!(res.is_consumed(), "wheel event must be consumed");
    assert!(
        nav.model_selector.selected_index() > before,
        "selection moved down: before={before} after={}",
        nav.model_selector.selected_index()
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Rapid wheel events ramp the scroll velocity
// ────────────────────────────────────────────────────────────────────────

#[test]
fn rapid_wheel_events_ramp_the_scroll_velocity() {
    // @step Given the /provider view is active with a list longer than the viewport
    let mut nav = provider_nav();
    let b = board();
    let start = nav.provider_settings.selected_index;

    // @step When several ScrollDown wheel events arrive in rapid succession
    let mut steps = 0usize;
    for _ in 0..5 {
        nav.handle_event(&wheel(MouseEventKind::ScrollDown), &b);
        steps += 1;
    }

    // @step Then the provider selection moves by more than one row
    let moved = nav.provider_settings.selected_index - start;
    assert!(
        moved > steps,
        "velocity ramp must move more than one row per event: moved={moved} over {steps} events"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: PageDown and PageUp page the provider list
// ────────────────────────────────────────────────────────────────────────

#[test]
fn page_down_and_page_up_page_the_provider_list() {
    // @step Given the /provider view is active with a list longer than the viewport
    let mut nav = provider_nav();
    let b = board();
    let start = nav.provider_settings.selected_index;

    // @step When I press PageDown and then PageUp
    nav.handle_event(&key_event(KeyCode::PageDown), &b);
    let after_down = nav.provider_settings.selected_index;
    nav.handle_event(&key_event(KeyCode::PageUp), &b);
    let after_up = nav.provider_settings.selected_index;

    // @step Then the selection moves down a page and then back up a page
    assert!(
        after_down >= start + 9,
        "PageDown advances ~one page: start={start} after_down={after_down}"
    );
    assert!(
        after_up < after_down,
        "PageUp moves back up: after_down={after_down} after_up={after_up}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Home and End jump to the ends of the provider list
// ────────────────────────────────────────────────────────────────────────

#[test]
fn home_and_end_jump_to_the_ends_of_the_provider_list() {
    // @step Given the /provider view is active with a list longer than the viewport
    let mut nav = provider_nav();
    let b = board();
    let last = nav.provider_settings.nav_items.len() - 1;

    // @step When I press End and then Home
    nav.handle_event(&key_event(KeyCode::End), &b);
    let after_end = nav.provider_settings.selected_index;
    nav.handle_event(&key_event(KeyCode::Home), &b);
    let after_home = nav.provider_settings.selected_index;

    // @step Then End selects the last item and Home selects the first item
    assert_eq!(after_end, last, "End selects last item");
    assert_eq!(after_home, 0, "Home selects first item");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Paging keys are inert in provider filter mode
// ────────────────────────────────────────────────────────────────────────

#[test]
fn paging_keys_are_inert_in_provider_filter_mode() {
    // @step Given the /provider view is active and filter mode is on
    let mut nav = provider_nav();
    let b = board();
    nav.handle_event(&key_event(KeyCode::Char('/')), &b);
    assert!(nav.provider_settings.filter_mode, "filter mode is on");
    let before = nav.provider_settings.selected_index;

    // @step When I press PageDown
    nav.handle_event(&key_event(KeyCode::PageDown), &b);

    // @step Then the provider selection does not move
    assert_eq!(
        nav.provider_settings.selected_index, before,
        "paging keys are inert while filtering"
    );
}
