//! PROV-145 — profile create/edit form UI coverage for the four new loop-
//! detection fields (10th-13th, after Max Images): the 'Loop Detection'
//! boolean toggle plus the 'Loop Window' / 'Loop Repeat' / 'Loop Retries'
//! numeric fields, plus the on-disk prefill read. Offline pure-state tests:
//! ProfileForm constructors + key-driven input through ProviderSettingsView
//! (no App, no backend, no filesystem except the path-injectable config
//! loader).
//!
//! Feature: spec/features/per-profile-loop-detection-form.feature
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the
//! code that exercises it.
//!
//! RED PHASE: `PROFILE_FORM_FIELDS` is still `[&str; 9]`, the `ProfileForm`
//! struct has no `loop_detection*` fields, and `profiles_config` does not
//! read the `loopDetection*` keys, so this target fails to compile until
//! the implementation lands.

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

fn form_of(view: &mut ProviderSettingsView) -> &mut ProfileForm {
    match &mut view.mode {
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

fn label_index(label: &str) -> usize {
    PROFILE_FORM_FIELDS
        .iter()
        .position(|l| *l == label)
        .expect("PROFILE_FORM_FIELDS must contain {label:?}")
}

/// A create form past the name step, focused on the field with the given label.
fn form_focused_on(label: &str) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.field_index = label_index(label);
    form
}

/// Type a string into the focused field one char at a time.
fn type_chars(view: &mut ProviderSettingsView, text: &str) {
    for c in text.chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
}

/// Scenario: The four Loop Detection fields appear after Max Images with the correct defaults
#[test]
fn the_four_loop_detection_fields_appear_after_max_images_with_the_correct_defaults() {
    // @step Given an OpenAI profile "work-vllm" exists with no loopDetection keys stored
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-stored".to_string(),
        ..ProfileDefinition::default()
    };

    // @step When I open the profile edit form in the /provider view
    let form = ProfileForm::from_definition("work-vllm", &def);

    // @step Then the "Loop Detection" field appears after "Max Images"
    let max_idx = label_index("Max Images");
    let ld_idx = label_index("Loop Detection");
    assert_eq!(
        ld_idx,
        max_idx + 1,
        "Loop Detection must be appended directly after Max Images"
    );

    // @step And the "Loop Window", "Loop Repeat", and "Loop Retries" fields appear after "Loop Detection"
    let lw_idx = label_index("Loop Window");
    let lr_idx = label_index("Loop Repeat");
    let lrt_idx = label_index("Loop Retries");
    assert_eq!(lw_idx, ld_idx + 1, "Loop Window must follow Loop Detection");
    assert_eq!(lr_idx, lw_idx + 1, "Loop Repeat must follow Loop Window");
    assert_eq!(lrt_idx, lr_idx + 1, "Loop Retries must follow Loop Repeat");

    // @step And the "Loop Detection" toggle prefills to Enabled (absent key preserves today's always-on behavior)
    assert!(
        form.loop_detection,
        "an absent loopDetectionEnabled key must prefill the toggle to enabled (today's always-on behavior)"
    );

    // @step And the "Loop Window" field is prefilled with 160
    assert_eq!(
        form.loop_window, "160",
        "an absent loopDetectionWindow key must prefill the effective default 160"
    );

    // @step And the "Loop Repeat" field is prefilled with 10
    assert_eq!(
        form.loop_repeat, "10",
        "an absent loopDetectionMaxRepeats key must prefill the effective default 10"
    );

    // @step And the "Loop Retries" field is prefilled with 10
    assert_eq!(
        form.loop_retries, "10",
        "an absent loopDetectionMaxRetries key must prefill the effective default 10"
    );
}

/// Scenario: Storing loop detection values prefills the fields on re-open
#[test]
fn storing_loop_detection_values_prefills_the_fields_on_re_open() {
    // @step Given an OpenAI profile "work-vllm" stores loopDetectionEnabled false, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    let def = ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-stored".to_string(),
        loop_detection_enabled: Some(false),
        loop_detection_window: Some(320),
        loop_detection_max_repeats: Some(5),
        loop_detection_max_retries: Some(2),
        ..ProfileDefinition::default()
    };

    // @step When I open the profile edit form in the /provider view
    let form = ProfileForm::from_definition("work-vllm", &def);

    // @step Then the "Loop Detection" toggle prefills to Disabled
    assert!(
        !form.loop_detection,
        "a stored false must prefill the toggle to disabled"
    );

    // @step And the "Loop Window" field is prefilled with 320
    assert_eq!(form.loop_window, "320");

    // @step And the "Loop Repeat" field is prefilled with 5
    assert_eq!(form.loop_repeat, "5");

    // @step And the "Loop Retries" field is prefilled with 2
    assert_eq!(form.loop_retries, "2");
}

/// Scenario: The Loop Detection toggle flips with Space and never accepts text
#[test]
fn the_loop_detection_toggle_flips_with_space_and_never_accepts_text() {
    // @step Given a profile form is open with the "Loop Detection" field focused
    let mut view = create_view(form_focused_on("Loop Detection"));

    // @step When I press Space
    view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the toggle flips to Disabled
    assert!(
        !form_of(&mut view).loop_detection,
        "Space must flip the toggle to disabled"
    );

    // @step When I press Space again
    view.handle_key(key(KeyCode::Char(' ')));

    // @step Then the toggle flips back to Enabled
    assert!(
        form_of(&mut view).loop_detection,
        "a second Space must flip the toggle back to enabled"
    );

    // @step And printable characters are never appended to the toggle field
    for c in ['x', 'y', 'z'] {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert!(
        form_of(&mut view).loop_detection,
        "printable characters must be swallowed, never appended"
    );
}

/// Scenario: Loop Window value saves and round-trips through the form
#[test]
fn loop_window_value_saves_and_round_trips_through_the_form() {
    // @step Given a profile form is open with the "Loop Window" field focused
    let mut view = create_view(form_focused_on("Loop Window"));

    // @step When I clear the field and type 320 and press save
    form_of(&mut view).loop_window.clear();
    type_chars(&mut view, "320");
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the built profile definition carries loopDetectionWindow 320
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    let def = match &event {
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile {
            definition,
            ..
        }) => definition.clone(),
        other => panic!("a valid form must emit SaveProfile, got {other:?}"),
    };
    assert_eq!(
        def.loop_detection_window,
        Some(320),
        "typing 320 and saving must produce loop_detection_window = Some(320)"
    );

    // @step And re-opening the form for the saved profile shows the "Loop Window" field prefilled with 320
    let reopened = ProfileForm::from_definition("local", &def);
    assert_eq!(reopened.loop_window, "320");
}

/// Scenario: Empty numeric loop-detection field saves as absent
#[test]
fn empty_numeric_loop_detection_field_saves_as_absent() {
    // @step Given an OpenAI profile "work-vllm" stores loopDetectionWindow 320
    let stored = ProfileDefinition {
        base_url: "http://h".to_string(),
        api_key: "sk".to_string(),
        loop_detection_window: Some(320),
        ..ProfileDefinition::default()
    };

    // @step When I open the profile edit form and clear the "Loop Window" field
    let mut form = ProfileForm::from_definition("work-vllm", &stored);
    assert_eq!(form.loop_window, "320", "prefill must show the stored 320");
    form.loop_window.clear();

    // @step And I press save
    let def = form
        .build_definition()
        .expect("an empty Loop Window field must build without a rejection hint")
        .expect("a valid form must build a definition");

    // @step Then the built profile definition has no loopDetectionWindow value
    assert_eq!(
        def.loop_detection_window, None,
        "an empty Loop Window field must build loop_detection_window = None (key removed on save)"
    );

    // @step And re-opening the form shows the "Loop Window" field prefilled with the default 160
    let reopened = ProfileForm::from_definition("work-vllm", &def);
    assert_eq!(
        reopened.loop_window, "160",
        "an absent loopDetectionWindow key must resolve back to the prefilled default 160"
    );
}

/// Scenario: Non-numeric Loop Window input rejects the save
#[test]
fn non_numeric_loop_window_input_rejects_the_save() {
    // @step Given a profile form is open with the "Loop Window" field focused
    let mut view = create_view(form_focused_on("Loop Window"));

    // @step When I type "abc" into the "Loop Window" field and press save
    type_chars(&mut view, "abc");
    assert_eq!(
        form_of(&mut view).loop_window,
        "abc",
        "the raw (invalid) text must be visible before the save attempt"
    );
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the save is rejected with a hint naming the Loop Window field
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    match &event {
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile { .. }) => {
            panic!("an invalid Loop Window value must NOT emit a SaveProfile action")
        }
        _ => {}
    }
    assert!(
        !view.status.is_empty() && view.status.contains("Loop Window"),
        "the view status must carry a hint naming the field, got: {:?}",
        view.status
    );

    // @step And nothing is persisted and the form stays open showing "abc"
    assert_eq!(
        form_of(&mut view).loop_window,
        "abc",
        "the form must stay open with the invalid value so the user can fix it"
    );
}

/// Scenario: Non-numeric Loop Repeat input rejects the save
#[test]
fn non_numeric_loop_repeat_input_rejects_the_save() {
    // @step Given a profile form is open with the "Loop Repeat" field focused
    let mut view = create_view(form_focused_on("Loop Repeat"));

    // @step When I type "x" into the "Loop Repeat" field and press save
    type_chars(&mut view, "x");
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the save is rejected with a hint naming the Loop Repeat field
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    match &event {
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile { .. }) => {
            panic!("an invalid Loop Repeat value must NOT emit a SaveProfile action")
        }
        _ => {}
    }
    assert!(
        !view.status.is_empty() && view.status.contains("Loop Repeat"),
        "the view status must carry a hint naming the field, got: {:?}",
        view.status
    );

    // @step And nothing is persisted and the form stays open showing "x"
    assert_eq!(
        form_of(&mut view).loop_repeat,
        "x",
        "the form must stay open with the invalid value so the user can fix it"
    );
}

/// Scenario: Non-numeric Loop Retries input rejects the save
#[test]
fn non_numeric_loop_retries_input_rejects_the_save() {
    // @step Given a profile form is open with the "Loop Retries" field focused
    let mut view = create_view(form_focused_on("Loop Retries"));

    // @step When I type "1.5" into the "Loop Retries" field and press save
    type_chars(&mut view, "1.5");
    let event = view.handle_key(key(KeyCode::Enter));

    // @step Then the save is rejected with a hint naming the Loop Retries field
    use codelet_fspec_tui::views::ProviderSettingsEvent;
    match &event {
        ProviderSettingsEvent::Emit(codelet_fspec_tui::components::Action::SaveProfile { .. }) => {
            panic!("an invalid Loop Retries value must NOT emit a SaveProfile action")
        }
        _ => {}
    }
    assert!(
        !view.status.is_empty() && view.status.contains("Loop Retries"),
        "the view status must carry a hint naming the field, got: {:?}",
        view.status
    );

    // @step And nothing is persisted and the form stays open showing "1.5"
    assert_eq!(
        form_of(&mut view).loop_retries,
        "1.5",
        "the form must stay open with the invalid value so the user can fix it"
    );
}

/// Supporting: the on-disk config read round-trips all four keys.
#[test]
fn config_loader_round_trips_the_loop_detection_keys() {
    // @step Given a config file with all four loop-detection keys on one profile and none on another
    let dir = tempfile::tempdir().expect("tempdir");
    let project_root = tempfile::tempdir().expect("temp project root");
    std::fs::write(
        dir.path().join("fspec-config.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "providers": { "openai": { "profiles": {
                "tuned": {
                    "baseUrl": "http://tuned",
                    "apiKey": "k",
                    "loopDetectionEnabled": false,
                    "loopDetectionWindow": 320,
                    "loopDetectionMaxRepeats": 5,
                    "loopDetectionMaxRetries": 2
                },
                "legacy": { "baseUrl": "http://legacy", "apiKey": "k" }
            } } }
        }))
        .expect("json"),
    )
    .expect("write");

    // @step When the full-config loader reads the profiles
    let profiles = load_openai_profile_configs_from(dir.path(), project_root.path());

    // @step Then the stored profile carries the stored values
    let tuned = &profiles["tuned"];
    assert_eq!(tuned.loop_detection_enabled, Some(false));
    assert_eq!(tuned.loop_detection_window, Some(320));
    assert_eq!(tuned.loop_detection_max_repeats, Some(5));
    assert_eq!(tuned.loop_detection_max_retries, Some(2));

    // @step And a profile without the keys loads with every field absent
    let legacy = &profiles["legacy"];
    assert_eq!(legacy.loop_detection_enabled, None);
    assert_eq!(legacy.loop_detection_window, None);
    assert_eq!(legacy.loop_detection_max_repeats, None);
    assert_eq!(legacy.loop_detection_max_retries, None);
}
