//! PROV-110 — profile create/edit form UI (state + view routing).
//!
//! Feature: spec/features/provider-settings-profile-form.feature
//!
//! Offline pure-state tests: ProfileForm constructors + key-driven editing
//! through ProviderSettingsView (no App, no backend, no filesystem). Mirrors
//! the TS profileFormModeHandler.ts / providerSettingsHelpers.ts behaviour.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::profile_form::{
    ProfileForm, DEFAULT_PROFILE_BASE_URL,
};
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::ProfileDefinition;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn typ(view: &mut ProviderSettingsView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

fn form_of(view: &ProviderSettingsView) -> &ProfileForm {
    match &view.mode {
        ProviderSettingsMode::CreateProfile { form, .. }
        | ProviderSettingsMode::EditProfile { form, .. } => form,
        other => panic!("expected a form mode, got {other:?}"),
    }
}

fn save_action(ev: ProviderSettingsEvent) -> Option<(String, String, ProfileDefinition)> {
    if let ProviderSettingsEvent::Emit(Action::SaveProfile {
        provider_id,
        profile_name,
        definition,
        ..
    }) = ev
    {
        Some((provider_id, profile_name, definition))
    } else {
        None
    }
}

/// Build a view sitting in CreateProfile mode with the given form.
fn create_view(form: ProfileForm) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.mode = ProviderSettingsMode::CreateProfile {
        provider_id: "openai".to_string(),
        form,
    };
    view
}

/// A create form past the name step, focused on a given field index.
fn form_on_field(name: &str, base_url: &str, api_key: &str, field_index: usize) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = name.to_string();
    form.base_url = base_url.to_string();
    form.api_key = api_key.to_string();
    form.field_index = field_index;
    form
}

#[test]
fn create_form_starts_editing_name_with_prefilled_base_url() {
    // @step Given a new create profile form for provider "openai"
    let form = ProfileForm::new_create();

    // @step Then the name editing flag is true
    assert!(form.is_editing_name);
    // @step And the name is empty
    assert_eq!(form.name, "");
    // @step And the base URL field shows "http://localhost:8888"
    assert_eq!(form.base_url, DEFAULT_PROFILE_BASE_URL);
    assert_eq!(form.base_url, "http://localhost:8888");
    // @step And the api key field is empty
    assert_eq!(form.api_key, "");
    // @step And the focused field index is 0
    assert_eq!(form.field_index, 0);
}

#[test]
fn edit_form_prefills_connection_fields_from_stored_profile() {
    // @step Given an edit profile form for provider "openai" profile "fireworks" with a stored definition
    let def = ProfileDefinition {
        base_url: "https://api.fireworks.ai/inference/v1".to_string(),
        api_key: "sk-stored".to_string(),
        context_window: Some(131072),
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
    };
    let form = ProfileForm::from_definition("fireworks", &def);

    // @step Then the name editing flag is false
    assert!(!form.is_editing_name);
    // @step And the name is "fireworks"
    assert_eq!(form.name, "fireworks");
    // @step And the base URL field shows the stored base URL
    assert_eq!(form.base_url, "https://api.fireworks.ai/inference/v1");
    // @step And the api key field shows the stored api key
    assert_eq!(form.api_key, "sk-stored");
    // @step And the focused field index is 0
    assert_eq!(form.field_index, 0);
}

#[test]
fn down_arrow_from_name_editing_focuses_first_connection_field() {
    // @step Given a new create profile form for provider "openai"
    let mut view = create_view(ProfileForm::new_create());

    // @step When the user presses the Down arrow key
    view.handle_key(key(KeyCode::Down));

    // @step Then the name editing flag is false
    assert!(!form_of(&view).is_editing_name);
    // @step And the focused field index is 0
    assert_eq!(form_of(&view).field_index, 0);
}

#[test]
fn up_arrow_on_first_field_re_enters_name_editing_in_create_mode() {
    // @step Given a create profile form for provider "openai" focused on the base URL field
    let mut view = create_view(form_on_field("", DEFAULT_PROFILE_BASE_URL, "", 0));

    // @step When the user presses the Up arrow key
    view.handle_key(key(KeyCode::Up));

    // @step Then the name editing flag is true
    assert!(form_of(&view).is_editing_name);
}

#[test]
fn tab_is_ignored_and_leaves_focused_field_unchanged() {
    // @step Given a create profile form for provider "openai" focused on the base URL field
    let mut view = create_view(form_on_field("", DEFAULT_PROFILE_BASE_URL, "", 0));

    // @step When the user presses the Tab key
    view.handle_key(key(KeyCode::Tab));

    // @step Then the focused field index is 0
    assert_eq!(form_of(&view).field_index, 0);
    // @step And the name editing flag is false
    assert!(!form_of(&view).is_editing_name);
}

#[test]
fn saving_valid_profile_emits_save_action_and_returns_to_list() {
    // @step Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    // focused on the api key field (index 1)
    let mut view = create_view(form_on_field("local", "http://localhost:8888", "", 1));

    // @step When the user types "sk-test" into the api key field
    typ(&mut view, "sk-test");
    // @step And the user presses the Enter key
    let ev = view.handle_key(key(KeyCode::Enter));

    // @step Then a SaveProfile action is emitted for provider "openai" profile "local"
    let (provider_id, profile_name, _def) = save_action(ev).expect("SaveProfile emitted");
    assert_eq!(provider_id, "openai");
    assert_eq!(profile_name, "local");
    // @step And the provider settings mode returns to list
    assert_eq!(view.mode, ProviderSettingsMode::List);
}

#[test]
fn saving_with_empty_api_key_does_nothing() {
    // @step Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    let mut view = create_view(form_on_field("local", "http://localhost:8888", "", 1));

    // @step When the user presses the Enter key
    let ev = view.handle_key(key(KeyCode::Enter));

    // @step Then no SaveProfile action is emitted
    assert!(save_action(ev).is_none());
    // @step And the provider settings mode stays on the form
    assert!(matches!(
        view.mode,
        ProviderSettingsMode::CreateProfile { .. }
    ));
}

#[test]
fn numeric_fields_are_parsed_on_save_and_non_numeric_input_is_omitted() {
    // @step Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888" and api key "sk-test"
    // focused on the context window field (index 2)
    let mut view = create_view(form_on_field(
        "local",
        "http://localhost:8888",
        "sk-test",
        2,
    ));

    // @step When the user types "128000" into the context window field
    typ(&mut view, "128000");
    // @step And the user presses the Enter key
    let ev = view.handle_key(key(KeyCode::Enter));

    let (_p, _n, def) = save_action(ev).expect("SaveProfile emitted");
    // @step Then the emitted definition context window is 128000
    assert_eq!(def.context_window, Some(128000));
    // @step And the emitted definition max output tokens is omitted
    assert_eq!(def.max_output_tokens, None);
}

#[test]
fn escape_returns_to_list_without_saving() {
    // @step Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888"
    let mut view = create_view(form_on_field(
        "local",
        "http://localhost:8888",
        "sk-test",
        1,
    ));

    // @step When the user presses the Escape key
    let ev = view.handle_key(key(KeyCode::Esc));

    // @step Then no SaveProfile action is emitted
    assert!(save_action(ev).is_none());
    // @step And the provider settings mode returns to list
    assert_eq!(view.mode, ProviderSettingsMode::List);
}

#[test]
fn compaction_threshold_percentage_is_parsed_on_save() {
    // @step Given a create profile form for provider "openai" with name "local" and base URL "http://localhost:8888" and api key "sk-test"
    // focused on the compaction threshold field (index 4)
    let mut view = create_view(form_on_field(
        "local",
        "http://localhost:8888",
        "sk-test",
        4,
    ));

    // @step When the user types "80%" into the compaction threshold field
    typ(&mut view, "80%");
    // @step And the user presses the Enter key
    let ev = view.handle_key(key(KeyCode::Enter));

    let (_p, _n, def) = save_action(ev).expect("SaveProfile emitted");
    // @step Then the emitted definition compaction threshold type is "percentage"
    assert_eq!(def.compaction_threshold_type.as_deref(), Some("percentage"));
    // @step And the emitted definition compaction threshold value is 80
    assert_eq!(def.compaction_threshold_value, Some(80));
}
