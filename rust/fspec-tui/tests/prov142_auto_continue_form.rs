// Feature: spec/features/profile-auto-continue-form.feature
//
// PROV-142 — profile create/edit form UI coverage for the new numeric
// "Auto-Continue" field (7th field, after Streaming). Offline pure-state
// tests: ProfileForm constructors + key-driven input through
// ProviderSettingsView (no App, no backend, no filesystem).
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

/// The Auto-Continue field's index into [`PROFILE_FORM_FIELDS`] (the 7th, last).
fn auto_continue_field_index() -> usize {
    PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Auto-Continue")
        .expect("PROFILE_FORM_FIELDS must contain an \"Auto-Continue\" entry")
}

/// A create form past the name step, focused on the Auto-Continue field.
fn form_focused_on_auto_continue() -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.field_index = auto_continue_field_index();
    form
}

/// Type a string into the focused field one char at a time.
fn type_chars(view: &mut ProviderSettingsView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

/// Scenario: New create-profile form seeds Auto-Continue to empty
#[test]
fn new_create_profile_form_seeds_auto_continue_to_empty() {
    // @step Given the user opens the create-profile form
    // @step When the form is initialized
    let form = ProfileForm::new_create();

    // @step Then the Auto-Continue field appears after the Streaming field
    let streaming_idx = PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Streaming")
        .expect("PROFILE_FORM_FIELDS must contain a \"Streaming\" entry");
    let auto_idx = auto_continue_field_index();
    assert_eq!(
        auto_idx,
        streaming_idx + 1,
        "Auto-Continue must be appended directly after Streaming"
    );

    // @step And the Auto-Continue field is empty with the placeholder hint "0 (off) or n (budget)"
    assert!(
        form.auto_continue.is_empty(),
        "a brand-new create form must seed Auto-Continue to empty"
    );
    let rendered = form.field_value(auto_idx);
    assert!(
        rendered.is_empty(),
        "the empty Auto-Continue field renders its (dim) placeholder, not a value"
    );
}

/// Scenario: Typing a budget in the Auto-Continue field and saving persists it
#[test]
fn typing_a_budget_in_the_auto_continue_field_and_saving_persists_it() {
    // @step Given the user is on the create-profile form with the Auto-Continue field focused
    let mut view = create_view(form_focused_on_auto_continue());

    // @step When the user types 300 and saves the profile
    type_chars(&mut view, "300");
    assert_eq!(
        form_of(&view).auto_continue,
        "300",
        "typed chars must land in the Auto-Continue field"
    );
    let def = form_of(&view)
        .build_definition()
        .expect("valid form must build without a rejection hint")
        .expect("valid form must build a definition");

    // @step Then the profile is saved with autoContinue set to 300
    assert_eq!(
        def.auto_continue,
        Some(300),
        "the built definition must carry auto_continue = Some(300)"
    );
}

/// Scenario: Typing 0 in the Auto-Continue field and saving persists explicit off
#[test]
fn typing_zero_in_the_auto_continue_field_and_saving_persists_explicit_off() {
    // @step Given the user is on the create-profile form with the Auto-Continue field focused
    let mut view = create_view(form_focused_on_auto_continue());

    // @step When the user types 0 and saves the profile
    type_chars(&mut view, "0");
    let def = form_of(&view)
        .build_definition()
        .expect("valid form must build without a rejection hint")
        .expect("valid form must build a definition");

    // @step Then the profile is saved with autoContinue set to 0
    assert_eq!(
        def.auto_continue,
        Some(0),
        "the explicit-off sentinel 0 must be preserved (not coerced to None)"
    );
}

/// Scenario: Editing a profile seeds Auto-Continue from the stored value
#[test]
fn editing_a_profile_seeds_auto_continue_from_the_stored_value() {
    // @step Given a stored profile whose autoContinue value is 500
    let def = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "sk-stored".to_string(),
        auto_continue: Some(500),
        ..ProfileDefinition::default()
    };

    // @step When the user opens that profile in the edit form
    let form = ProfileForm::from_definition("stored", &def);

    // @step Then the Auto-Continue field shows 500
    assert_eq!(
        form.auto_continue, "500",
        "edit form must prefill Auto-Continue from the stored value"
    );
}

/// Scenario: Editing a profile without an autoContinue key seeds Auto-Continue to empty
#[test]
fn editing_a_profile_without_auto_continue_key_seeds_field_to_empty() {
    // @step Given a stored profile with no autoContinue key
    let def = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "sk-stored".to_string(),
        auto_continue: None,
        ..ProfileDefinition::default()
    };

    // @step When the user opens that profile in the edit form
    let form = ProfileForm::from_definition("stored", &def);

    // @step Then the Auto-Continue field is empty with the placeholder hint "0 (off) or n (budget)"
    assert!(
        form.auto_continue.is_empty(),
        "a profile with no autoContinue key must seed the field empty"
    );
}

/// Scenario: Non-numeric input in the Auto-Continue field rejects the save
#[test]
fn non_numeric_input_in_the_auto_continue_field_rejects_the_save() {
    // @step Given the user is on the profile form with the Auto-Continue field focused
    let mut view = create_view(form_focused_on_auto_continue());

    // @step When the user types abc and saves the profile
    type_chars(&mut view, "abc");
    assert_eq!(
        form_of(&view).auto_continue,
        "abc",
        "the raw (invalid) text must be visible before the save attempt"
    );
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the save is rejected with a hint that the value must be 0 or a positive integer
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    let rejected = match &event {
        // No SaveProfile action may be emitted for an invalid value.
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile {
            ..
        }) => {
            panic!("an invalid Auto-Continue value must NOT emit a SaveProfile action")
        }
        _ => true,
    };
    assert!(
        rejected,
        "the save must be rejected (no SaveProfile emitted)"
    );
    assert!(
        !view.status.is_empty()
            && view.status.contains("Auto-Continue")
            && view.status.contains("0"),
        "the view status must carry a hint naming the field and the valid range, got: {:?}",
        view.status
    );

    // @step And the profile is not modified on disk
    // (No backend round-trip is dispatched: the form stays open in the same
    // mode with the invalid text still present.)
    assert_eq!(
        form_of(&view).auto_continue,
        "abc",
        "the form must stay open with the invalid value so the user can fix it"
    );
}
