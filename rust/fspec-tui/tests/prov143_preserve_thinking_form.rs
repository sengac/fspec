// Feature: spec/features/profile-preserve-thinking-form.feature
//
// PROV-143 — profile create/edit form UI coverage for the new boolean
// "Preserve Thinking" toggle (8th field, after Auto-Continue). Offline
// pure-state tests: ProfileForm constructors + key-driven input through
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

/// The Preserve Thinking field's index into [`PROFILE_FORM_FIELDS`] (the 8th, last).
fn preserve_thinking_field_index() -> usize {
    PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Preserve Thinking")
        .expect("PROFILE_FORM_FIELDS must contain a \"Preserve Thinking\" entry")
}

/// A create form past the name step, focused on the Preserve Thinking field.
fn form_focused_on_preserve_thinking() -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.field_index = preserve_thinking_field_index();
    form
}

/// Scenario: The Preserve Thinking toggle appears after Auto-Continue
#[test]
fn preserve_thinking_toggle_appears_after_auto_continue() {
    // @step Given the profile form field list is rendered
    // @step When the form is inspected
    let auto_idx = PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Auto-Continue")
        .expect("PROFILE_FORM_FIELDS must contain an \"Auto-Continue\" entry");
    let pt_idx = preserve_thinking_field_index();

    // @step Then "Preserve Thinking" is the 8th (last) field
    assert_eq!(
        pt_idx,
        auto_idx + 1,
        "Preserve Thinking must be appended directly after Auto-Continue"
    );
    assert_eq!(
        pt_idx,
        PROFILE_FORM_FIELDS.len() - 1,
        "Preserve Thinking must be the last form field"
    );

    // @step And the focused-field routing treats it as a boolean toggle like Streaming
    // Pressing a printable char on the focused Preserve Thinking field must be
    // swallowed (boolean routing), never appended to any text field.
    let mut view = create_view(form_focused_on_preserve_thinking());
    view.handle_key(key(KeyCode::Char('x')));
    assert!(
        !form_of(&view).preserve_thinking,
        "boolean routing must swallow printable keys, not append them"
    );
}

/// Scenario: A new profile defaults Preserve Thinking to disabled
#[test]
fn new_profile_defaults_preserve_thinking_to_disabled() {
    // @step Given a brand-new profile form is created
    let form = ProfileForm::new_create();

    // @step When the form is inspected
    // @step Then preserve_thinking is false
    assert!(
        !form.preserve_thinking,
        "a brand-new profile must default Preserve Thinking to disabled"
    );

    // @step And the display value for the field is "Disabled"
    assert_eq!(
        form.field_value(preserve_thinking_field_index()),
        "Disabled",
        "the disabled default must render the 'Disabled' label"
    );
}

/// Scenario: Toggling the field flips the boolean
#[test]
fn toggling_preserve_thinking_flips_the_boolean() {
    // @step Given the Preserve Thinking field is focused
    let mut view = create_view(form_focused_on_preserve_thinking());

    // @step When Space is pressed
    view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the value becomes true and renders "Enabled"
    assert!(form_of(&view).preserve_thinking, "Space must enable the toggle");
    assert_eq!(
        form_of(&view).field_value(preserve_thinking_field_index()),
        "Enabled",
        "the enabled value must render the 'Enabled' label"
    );

    // @step When Space is pressed again
    view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the value becomes false and renders "Disabled"
    assert!(!form_of(&view).preserve_thinking, "Space must flip the toggle back");
    assert_eq!(
        form_of(&view).field_value(preserve_thinking_field_index()),
        "Disabled",
    );

    // @step And printable characters are never appended to the field
    for c in ['x', 'y', 'z'] {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert!(
        !form_of(&view).preserve_thinking,
        "printable characters must be swallowed, never appended"
    );
}

/// Scenario: Editing a profile prefills the stored value
#[test]
fn editing_a_profile_prefills_the_stored_value() {
    // @step Given a stored profile with preserveThinking = true
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk".to_string(),
        preserve_thinking: Some(true),
        ..ProfileDefinition::default()
    };

    // @step When the edit form is opened for that profile
    let form = ProfileForm::from_definition("local", &def);

    // @step Then preserve_thinking is true and renders "Enabled"
    assert!(form.preserve_thinking, "stored true must prefill the toggle");
    assert_eq!(
        form.field_value(preserve_thinking_field_index()),
        "Enabled",
    );

    // @step And a stored profile with the key absent seeds preserve_thinking false
    let def_absent = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk".to_string(),
        preserve_thinking: None,
        ..ProfileDefinition::default()
    };
    let form_absent = ProfileForm::from_definition("local", &def_absent);
    assert!(
        !form_absent.preserve_thinking,
        "an absent key must seed the toggle to disabled"
    );
}

/// Scenario: Saving a profile persists the toggle
#[test]
fn saving_a_profile_persists_the_toggle() {
    // @step Given a profile form with preserve_thinking = true
    let mut form = form_focused_on_preserve_thinking();
    form.preserve_thinking = true;

    // @step When the form is built into a ProfileDefinition
    let built = form
        .build_definition()
        .expect("valid form must build without a rejection hint")
        .expect("valid form must build a definition");

    // @step Then the definition carries preserve_thinking = Some(true)
    assert_eq!(built.preserve_thinking, Some(true));
}

#[test]
fn saving_a_profile_with_the_toggle_off_persists_some_false() {
    // @step Given a profile form with preserve_thinking = false
    let form = form_focused_on_preserve_thinking();

    // @step When the form is built into a ProfileDefinition
    let built = form
        .build_definition()
        .expect("valid form must build without a rejection hint")
        .expect("valid form must build a definition");

    // @step Then the definition carries preserve_thinking = Some(false)
    assert_eq!(built.preserve_thinking, Some(false));
}

/// Scenario: The config loader round-trips the preserveThinking flag
#[test]
fn config_loader_round_trips_the_preserve_thinking_flag() {
    // @step Given a config file with preserveThinking = true on one profile and an absent key on another
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("fspec-config.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(
            &serde_json::json!({
                "providers": { "openai": { "profiles": {
                    "on":  { "baseUrl": "http://on",  "apiKey": "k", "preserveThinking": true },
                    "off": { "baseUrl": "http://off", "apiKey": "k" }
                } } }
            }),
        )
        .expect("json"),
    )
    .expect("write");

    // @step When the full-config loader reads the profiles
    let profiles = codelet_fspec_tui::views::provider_settings::profiles_config::
        load_openai_profile_configs_from(dir.path(), dir.path());

    // @step Then the stored profile preserves the stored value
    assert_eq!(profiles["on"].preserve_thinking, Some(true));

    // @step And an absent key seeds None (=> disabled)
    assert_eq!(profiles["off"].preserve_thinking, None);
}
