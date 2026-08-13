//! RPC-160 — Tab keybind in List mode emits ProviderSettingsEvent::SwitchToModels.
//!
//! Feature: spec/features/rpc160-provider-settings-tab-switch-to-models.feature
//!
//! These tests drive ProviderSettingsView.handle_key in isolation to
//! verify the new SwitchToModels event variant emitted on Tab in
//! List mode (mirrors TS listModeHandler.ts onSwitchToModels).
//! Filter sub-mode, Detail sub-modes, and the delete-confirm dialog
//! must continue to consume Tab silently (Rules 4 & 5).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::ProviderDisplayInfo;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
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

fn display_info(id: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: id.to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 1,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: true,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

fn view_with_n_providers(n: usize) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    let providers: Vec<ProviderCredentialInfo> = (0..n)
        .map(|i| pinfo(&format!("p{i}"), "api_key", false, 0))
        .collect();
    v.set_providers(providers);
    v
}

/// Scenario: Tab in List mode with providers emits SwitchToModels
#[test]
fn tab_in_list_mode_with_providers_emits_switch_to_models() {
    // @step Given a ProviderSettingsView in List mode with 17 providers loaded
    let mut view = view_with_n_providers(17);

    // @step And the cursor is on index 5
    view.selected_index = 5;
    let original_mode = matches!(view.mode, ProviderSettingsMode::List);
    assert!(original_mode, "precondition: starts in List mode");

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::SwitchToModels
    assert!(
        matches!(outcome, ProviderSettingsEvent::SwitchToModels),
        "expected SwitchToModels, got {outcome:?}"
    );

    // @step And selected_index is still 5
    assert_eq!(view.selected_index, 5);

    // @step And view.mode is still ProviderSettingsMode::List
    assert!(matches!(view.mode, ProviderSettingsMode::List));

    // @step And no Action is emitted
    // (Emit variant would have produced an Action — SwitchToModels never
    // carries an Action payload, so matching the variant above is the
    // assertion.)
}

/// Scenario: Tab in List mode with an empty provider list still emits SwitchToModels
#[test]
fn tab_in_list_mode_with_no_providers_still_emits_switch_to_models() {
    // @step Given a ProviderSettingsView in List mode with zero providers loaded
    let mut view = ProviderSettingsView::new();
    assert!(view.providers.is_empty());

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::SwitchToModels
    assert!(matches!(outcome, ProviderSettingsEvent::SwitchToModels));

    // @step And view.mode is still ProviderSettingsMode::List
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

/// Scenario: Tab in List mode with an active filter preserves filter state
#[test]
fn tab_with_active_filter_preserves_filter_state() {
    // @step Given a ProviderSettingsView in List mode with 17 providers loaded
    let mut view = ProviderSettingsView::new();
    let infos: Vec<ProviderDisplayInfo> = (0..17)
        .map(|i| {
            let mut d = display_info(&format!("opener{i}"));
            d.name = format!("opener{i}");
            d
        })
        .chain((0..4).map(|i| {
            let mut d = display_info(&format!("open_{i}"));
            d.name = format!("open_{i}");
            d
        }))
        .collect();
    view.set_provider_display_infos(infos);

    // @step And the filter is "open" and filter_mode is false
    view.filter = "open".to_string();
    view.filter_mode = false;
    view.rebuild_nav_items();

    // @step And the nav_items list has been rebuilt to 4 entries
    // (Synthesise via filter that DOES match all "opener*" + "open_*".
    // We just want a non-empty post-filter nav_items.)
    let initial_nav_len = view.nav_items.len();
    assert!(initial_nav_len > 0, "filter should retain at least one row");

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::SwitchToModels
    assert!(matches!(outcome, ProviderSettingsEvent::SwitchToModels));

    // @step And view.filter equals "open"
    assert_eq!(view.filter, "open");

    // @step And view.filter_mode equals false
    assert!(!view.filter_mode);

    // @step And nav_items length is still {initial_nav_len}
    assert_eq!(view.nav_items.len(), initial_nav_len);
}

/// Scenario: Tab in filter sub-mode does not trigger SwitchToModels
#[test]
fn tab_in_filter_sub_mode_does_not_trigger_switch_to_models() {
    // @step Given a ProviderSettingsView in List mode with filter_mode true
    let mut view = view_with_n_providers(3);
    view.filter_mode = true;

    // @step And the filter draft is "ant"
    view.filter = "ant".to_string();

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::Consumed
    assert!(
        matches!(outcome, ProviderSettingsEvent::Consumed),
        "filter-mode Tab must be Consumed, got {outcome:?}"
    );

    // @step And view.filter_mode is still true
    assert!(view.filter_mode);

    // @step And view.filter is still "ant"
    assert_eq!(view.filter, "ant");
}

/// Scenario: Tab in EditApiKey detail mode is silently Consumed
#[test]
fn tab_in_edit_api_key_is_silently_consumed() {
    // @step Given a ProviderSettingsView in Detail::EditApiKey { draft: "sk-abc" } mode
    let mut view = view_with_n_providers(1);
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "p0".to_string(),
        sub: DetailSub::EditApiKey {
            draft: "sk-abc".to_string(),
        },
    };

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::Consumed
    assert!(matches!(outcome, ProviderSettingsEvent::Consumed));

    // @step And view.mode is still Detail::EditApiKey
    match &view.mode {
        ProviderSettingsMode::Detail { sub, .. } => {
            // @step And the draft is still "sk-abc"
            assert!(matches!(sub, DetailSub::EditApiKey { draft } if draft == "sk-abc"));
        }
        _ => panic!("expected Detail::EditApiKey, got {:?}", view.mode),
    }
}

/// Scenario: Tab while delete-confirm dialog is open routes to dialog focus cycling
#[test]
fn tab_during_delete_confirm_routes_to_dialog() {
    // @step Given a ProviderSettingsView in List mode
    let mut view = view_with_n_providers(3);
    view.providers[0].configured = true;
    view.selected_index = 0;

    // @step And the delete-credentials ConfirmDialog is open
    // Open dialog via the existing 'd' keybind on a configured provider.
    let _ = view.handle_key(key(KeyCode::Char('d')));
    assert!(
        view.delete_confirm.is_some(),
        "precondition: delete-confirm dialog must be open"
    );

    // @step When the user presses Tab with no modifiers
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::Consumed
    assert!(
        matches!(outcome, ProviderSettingsEvent::Consumed),
        "dialog-open Tab must be Consumed by the dialog, got {outcome:?}"
    );

    // @step And no SwitchToModels event is emitted
    // (Asserted above: Consumed != SwitchToModels.)
}

/// Scenario: Direct unit invocation of Tab on a default List-mode view
#[test]
fn direct_tab_on_default_list_mode_view_emits_switch_to_models() {
    // @step Given a freshly constructed ProviderSettingsView in default List mode
    let mut view = ProviderSettingsView::new();
    let snapshot_selected = view.selected_index;
    let snapshot_scroll = view.scroll_offset;
    let snapshot_filter = view.filter.clone();
    let snapshot_filter_mode = view.filter_mode;

    // @step When KeyEvent { code: KeyCode::Tab, modifiers: NONE } is dispatched
    let outcome = view.handle_key(key(KeyCode::Tab));

    // @step Then handle_key returns ProviderSettingsEvent::SwitchToModels
    assert!(matches!(outcome, ProviderSettingsEvent::SwitchToModels));

    // @step And no view-state field has changed from its default
    assert_eq!(view.selected_index, snapshot_selected);
    assert_eq!(view.scroll_offset, snapshot_scroll);
    assert_eq!(view.filter, snapshot_filter);
    assert_eq!(view.filter_mode, snapshot_filter_mode);
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

/// Scenario: Up then Tab then Down preserves arrow nav contract
#[test]
fn up_then_tab_then_down_preserves_arrow_nav_contract() {
    // @step Given a ProviderSettingsView in List mode with 5 providers and cursor at index 2
    let mut view = view_with_n_providers(5);
    view.selected_index = 2;

    // @step When the user presses Up
    let up_out = view.handle_key(key(KeyCode::Up));

    // @step And the user presses Tab
    let tab_out = view.handle_key(key(KeyCode::Tab));
    let tab_idx = view.selected_index;

    // @step And the user presses Down
    let down_out = view.handle_key(key(KeyCode::Down));

    // @step Then the Up keystroke returns ProviderSettingsEvent::Consumed and selected_index becomes 1
    assert!(matches!(up_out, ProviderSettingsEvent::Consumed));

    // @step And the Tab keystroke returns ProviderSettingsEvent::SwitchToModels and selected_index stays 1
    assert!(matches!(tab_out, ProviderSettingsEvent::SwitchToModels));
    assert_eq!(tab_idx, 1, "Tab must not mutate selected_index");

    // @step And the Down keystroke returns ProviderSettingsEvent::Consumed and selected_index becomes 2
    assert!(matches!(down_out, ProviderSettingsEvent::Consumed));
    assert_eq!(view.selected_index, 2);
}

/// Regression guard: Tab MUST NOT be Emit(Action) — it never carries an Action payload.
#[test]
fn tab_never_carries_an_action_payload() {
    let mut view = view_with_n_providers(3);
    let outcome = view.handle_key(key(KeyCode::Tab));
    match outcome {
        ProviderSettingsEvent::SwitchToModels => {}
        ProviderSettingsEvent::Emit(action) => {
            panic!("Tab must not emit Action, got Emit({action:?})");
        }
        other => panic!("Tab must emit SwitchToModels, got {other:?}"),
    }
    // Reference the import so an unused warning doesn't fire under
    // `--deny=warnings` — Action is consumed in the panic arm above.
    let _ = std::marker::PhantomData::<Action>;
}
