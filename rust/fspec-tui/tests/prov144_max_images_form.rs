//! PROV-144 — profile create/edit form UI coverage for the new numeric
//! "Max Images" field (9th field, after Preserve Thinking) plus the
//! on-disk prefill read. Offline pure-state tests: ProfileForm
//! constructors + key-driven input through ProviderSettingsView (no App,
//! no backend, no filesystem except the path-injectable config loader).
//!
//! Feature: spec/features/per-profile-max-images-form.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it.
//!
//! RED PHASE: `PROFILE_FORM_FIELDS` is still `[&str; 8]`, the `ProfileForm`
//! struct has no `max_images` field, and `profiles_config` does not read the
//! `maxImages` key, so this target fails to compile until the
//! implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::profile_form::{ProfileForm, PROFILE_FORM_FIELDS};
use codelet_fspec_tui::views::provider_settings::profiles_config::load_openai_profile_configs_from;
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

/// The Max Images field's index into [`PROFILE_FORM_FIELDS`] (the 9th, last).
fn max_images_field_index() -> usize {
    PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Max Images")
        .expect("PROFILE_FORM_FIELDS must contain a \"Max Images\" entry")
}

/// A create form past the name step, focused on the Max Images field.
fn form_focused_on_max_images() -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.field_index = max_images_field_index();
    form
}

/// Type a string into the focused field one char at a time.
fn type_chars(view: &mut ProviderSettingsView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

/// Scenario: Max Images field prefills to the default 4 when absent
#[test]
fn max_images_field_prefills_to_the_default_4_when_absent() {
    // @step Given an OpenAI profile "work-vllm" exists with no maxImages key stored
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-stored".to_string(),
        max_images: None,
        ..ProfileDefinition::default()
    };

    // @step When I open the profile edit form in the /provider view
    let form = ProfileForm::from_definition("work-vllm", &def);

    // @step Then the "Max Images" field appears after "Preserve Thinking"
    let pt_idx = PROFILE_FORM_FIELDS
        .iter()
        .position(|label| *label == "Preserve Thinking")
        .expect("PROFILE_FORM_FIELDS must contain a \"Preserve Thinking\" entry");
    let max_idx = max_images_field_index();
    assert_eq!(
        max_idx,
        pt_idx + 1,
        "Max Images must be appended directly after Preserve Thinking"
    );

    // @step And the "Max Images" field is prefilled with 4
    assert_eq!(
        form.max_images, "4",
        "an absent maxImages key must prefill the field with the default 4"
    );

    // @step When I type 2 into the "Max Images" field and press save
    let mut form = form;
    form.max_images.clear();
    form.field_index = max_idx;
    let mut view = create_view(form);
    type_chars(&mut view, "2");
    let def = form_of(&view)
        .build_definition()
        .expect("a valid form must build without a rejection hint")
        .expect("a valid form must build a definition");

    // @step Then the profile "work-vllm" is stored with maxImages 2
    assert_eq!(
        def.max_images,
        Some(2),
        "typing 2 and saving must produce max_images = Some(2)"
    );

    // @step And re-opening the form shows the "Max Images" field prefilled with 2
    let reopened = ProfileForm::from_definition("work-vllm", &def);
    assert_eq!(reopened.max_images, "2");
}

/// Scenario: Empty Max Images field saves as absent and resolves to the default
#[test]
fn empty_max_images_field_saves_as_absent_and_resolves_to_the_default() {
    // @step Given an OpenAI profile "work-vllm" stores maxImages 2
    let stored = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "sk".to_string(),
        max_images: Some(2),
        ..ProfileDefinition::default()
    };

    // @step When I open the profile edit form and clear the "Max Images" field
    let mut form = ProfileForm::from_definition("work-vllm", &stored);
    assert_eq!(form.max_images, "2", "prefill must show the stored 2");
    form.max_images.clear();

    // @step And I press save
    let def = form
        .build_definition()
        .expect("an empty Max Images field must build without a rejection hint")
        .expect("a valid form must build a definition");

    // @step Then the profile "work-vllm" has no maxImages key on disk
    // (None is the "remove the key" value in the persistence read-modify-write)
    assert_eq!(
        def.max_images, None,
        "an empty Max Images field must build max_images = None (key removed on save)"
    );

    // @step And re-opening the form shows the "Max Images" field prefilled with 4
    let reopened = ProfileForm::from_definition("work-vllm", &def);
    assert_eq!(
        reopened.max_images, "4",
        "an absent maxImages key must resolve back to the prefilled default 4"
    );
}

/// Scenario: Non-numeric Max Images input rejects the save
#[test]
fn non_numeric_max_images_input_rejects_the_save() {
    // @step Given a profile edit form is open
    let mut view = create_view(form_focused_on_max_images());

    // @step When I type "abc" into the "Max Images" field and press save
    type_chars(&mut view, "abc");
    assert_eq!(
        form_of(&view).max_images,
        "abc",
        "the raw (invalid) text must be visible before the save attempt"
    );
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the save is rejected with a hint that Max Images must be a whole number (0 = no vision, 4 = default)
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    let rejected = match &event {
        // No SaveProfile action may be emitted for an invalid value.
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile {
            ..
        }) => {
            panic!("an invalid Max Images value must NOT emit a SaveProfile action")
        }
        _ => true,
    };
    assert!(
        rejected,
        "the save must be rejected (no SaveProfile emitted)"
    );
    assert!(
        !view.status.is_empty()
            && view.status.contains("Max Images")
            && view.status.contains("whole number"),
        "the view status must carry a hint naming the field and the valid range, got: {:?}",
        view.status
    );

    // @step And nothing is persisted
    // (No backend round-trip is dispatched: the form stays open in the same
    // mode with the invalid text still present.)
    assert_eq!(
        form_of(&view).max_images,
        "abc",
        "the form must stay open with the invalid value so the user can fix it"
    );
}

/// Scenario: The maxImages value round-trips through wire and disk (disk read)
#[test]
fn the_max_images_value_round_trips_through_the_on_disk_read() {
    // @step Given a profile definition with maxImages 7
    let user_dir = tempfile::tempdir().expect("temp user dir");
    let project_root = tempfile::tempdir().expect("temp project root");

    // @step When the profile is saved to fspec-config.json
    std::fs::write(
        user_dir.path().join("fspec-config.json"),
        serde_json::json!({
            "providers": {
                "openai": {
                    "profiles": {
                        "work-vllm": {
                            "baseUrl": "http://192.168.0.50:8000",
                            "apiKey": "sk-test",
                            "maxImages": 7
                        }
                    }
                }
            }
        })
        .to_string(),
    )
    .expect("write fspec-config.json");

    // @step Then the stored profile object contains "maxImages": 7
    let configs = load_openai_profile_configs_from(user_dir.path(), project_root.path());
    let def = configs
        .get("work-vllm")
        .expect("the profile must be loaded from disk");
    assert_eq!(
        def.max_images,
        Some(7),
        "the on-disk maxImages key must round-trip into the wire field"
    );

    // @step And re-reading the profile resolves the effective limit to 7
    assert_eq!(
        def.max_images_limit(),
        7,
        "the effective limit predicate must resolve 7"
    );
}

/// Scenario: A missing maxImages key resolves to the default 4 (disk read)
#[test]
fn a_missing_max_images_key_resolves_to_the_default_4_on_read() {
    // @step Given a profile definition without a maxImages field
    let user_dir = tempfile::tempdir().expect("temp user dir");
    let project_root = tempfile::tempdir().expect("temp project root");

    // @step When the profile is saved to fspec-config.json
    std::fs::write(
        user_dir.path().join("fspec-config.json"),
        serde_json::json!({
            "providers": {
                "openai": {
                    "profiles": {
                        "legacy": {
                            "baseUrl": "http://192.168.0.50:8000",
                            "apiKey": "sk-test"
                        }
                    }
                }
            }
        })
        .to_string(),
    )
    .expect("write fspec-config.json");

    // @step Then the stored profile object has no maxImages key
    let configs = load_openai_profile_configs_from(user_dir.path(), project_root.path());
    let def = configs
        .get("legacy")
        .expect("the profile must be loaded from disk");
    assert_eq!(
        def.max_images, None,
        "a profile without the maxImages key must load with max_images = None"
    );

    // @step And re-reading the profile resolves the effective limit to 4
    // The form prefill applies the effective default:
    let form = ProfileForm::from_definition("legacy", def);
    assert_eq!(form.max_images, "4", "absent key ⇒ effective default 4");
}
