// Feature: spec/features/provider-settings-profile-streaming.feature
//
// PROV-139 — profile create/edit form UI coverage for the Streaming boolean
// toggle field. Offline pure-state tests: ProfileForm constructors + key-driven
// toggling through ProviderSettingsView (no App, no backend, no filesystem).
//
// Each Gherkin step maps to a `// @step` comment immediately above the code
// that exercises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::profile_form::{ProfileForm, PROFILE_FORM_FIELDS};
use codelet_fspec_tui::views::{ProviderSettingsMode, ProviderSettingsView};
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

fn form_of(view: &ProviderSettingsView) -> &ProfileForm {
    match &view.mode {
        ProviderSettingsMode::CreateProfile { form, .. }
        | ProviderSettingsMode::EditProfile { form, .. } => form,
        other => panic!("expected a form mode, got {other:?}"),
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

/// The Streaming field's index into [`PROFILE_FORM_FIELDS`] (the 6th, last).
fn streaming_field_index() -> usize {
    PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Streaming")
        .expect("PROFILE_FORM_FIELDS must contain a \"Streaming\" entry")
}

/// A create form past the name step, focused on the Streaming field.
fn form_focused_on_streaming(streaming: bool) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.streaming = streaming;
    form.field_index = streaming_field_index();
    form
}

/// Scenario: New create-profile form seeds Streaming to enabled
#[test]
fn new_create_profile_form_seeds_streaming_to_enabled() {
    // @step Given the user opens the create-profile form
    // @step When the form is initialized
    let form = ProfileForm::new_create();

    // @step Then the Streaming field shows Enabled
    assert!(
        form.streaming,
        "a brand-new create form must seed Streaming to enabled (true)"
    );
}

/// Scenario: Space toggles the Streaming field
#[test]
fn space_toggles_the_streaming_field() {
    // @step Given the user is on the create-profile form with the Streaming field focused
    let mut view = create_view(form_focused_on_streaming(true));
    assert!(form_of(&view).streaming, "precondition: Streaming enabled");

    // @step When the user presses Space
    view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the Streaming field flips from Enabled to Disabled
    assert!(
        !form_of(&view).streaming,
        "Space must flip Streaming from enabled to disabled"
    );
}

/// Scenario: Typing a printable character does not mutate the Streaming field
#[test]
fn typing_a_printable_character_does_not_mutate_the_streaming_field() {
    // @step Given the user is on the create-profile form with the Streaming field focused and Streaming enabled
    let mut view = create_view(form_focused_on_streaming(true));

    // @step When the user types the letter x
    view.handle_key(key(KeyCode::Char('x')));

    // @step Then the Streaming field stays Enabled with no text appended
    let form = form_of(&view);
    assert!(
        form.streaming,
        "typing a printable char must not toggle Streaming off"
    );
    // The Streaming field is a boolean, not a text field: no backing string
    // may have grown from the 'x'.
    assert!(
        !form.base_url.contains('x')
            && !form.api_key.contains('x')
            && !form.context_window.contains('x')
            && !form.max_output_tokens.contains('x')
            && !form.compaction_threshold.contains('x'),
        "the 'x' must not have been appended to any text field"
    );
}

/// Scenario: Editing a profile seeds Streaming from the stored definition
#[test]
fn editing_a_profile_seeds_streaming_from_the_stored_definition() {
    // @step Given a stored profile whose streaming flag is set to disabled
    let def = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "sk-stored".to_string(),
        streaming: Some(false),
        ..ProfileDefinition::default()
    };

    // @step When the user opens that profile in the edit form
    let form = ProfileForm::from_definition("stored", &def);

    // @step Then the Streaming field shows Disabled
    assert!(
        !form.streaming,
        "edit form must seed Streaming from the stored disabled definition"
    );
}

/// Scenario: build_definition emits the current toggle value
#[test]
fn build_definition_emits_the_current_toggle_value() {
    // @step Given the user is on the profile form with Streaming toggled to disabled
    let form = form_focused_on_streaming(false);

    // @step When the form builds a profile definition
    let def = form
        .build_definition()
        .expect("valid form must build without a rejection hint")
        .expect("valid form must build a definition");

    // @step Then the built definition carries streaming set to disabled
    assert_eq!(def.streaming, Some(false));
}
