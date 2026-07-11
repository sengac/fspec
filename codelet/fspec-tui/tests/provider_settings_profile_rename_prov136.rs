//! PROV-136 — provider-settings profile RENAME (edit-mode name editing +
//! rename emission).
//!
//! Feature: spec/features/provider-settings-profile-rename.feature
//!
//! Offline pure-state tests: an EDIT-mode ProfileForm driven through
//! ProviderSettingsView key routing (no App, no backend, no filesystem).
//! Covers the deliberate divergence from the TS reference: in edit mode the
//! name field is editable and Up from the first connection field re-enters it,
//! and a rename emits the ORIGINAL name so the persistence layer can delete the
//! old key. Persistence-layer rename coverage lives in the codelet-sessions
//! `profile_persistence_tests.rs` unit module.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
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

fn form_of(view: &ProviderSettingsView) -> &ProfileForm {
    match &view.mode {
        ProviderSettingsMode::CreateProfile { form, .. }
        | ProviderSettingsMode::EditProfile { form, .. } => form,
        other => panic!("expected a form mode, got {other:?}"),
    }
}

fn save_action(ev: ProviderSettingsEvent) -> Option<(String, String, Option<String>)> {
    if let ProviderSettingsEvent::Emit(Action::SaveProfile {
        provider_id,
        profile_name,
        old_profile_name,
        ..
    }) = ev
    {
        Some((provider_id, profile_name, old_profile_name))
    } else {
        None
    }
}

fn stored_def() -> ProfileDefinition {
    ProfileDefinition {
        base_url: "http://localhost:8888".to_string(),
        api_key: "sk-stored".to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        streaming: None,
    }
}

/// Build a view sitting in EditProfile mode for `profile_name`, with the form
/// prefilled from the stored definition and focused on the first connection
/// field (Base URL, index 0) — the state right after opening the edit form.
fn edit_view(profile_name: &str) -> ProviderSettingsView {
    let form = ProfileForm::from_definition(profile_name, &stored_def());
    let mut view = ProviderSettingsView::new();
    view.mode = ProviderSettingsMode::EditProfile {
        provider_id: "openai".to_string(),
        profile_name: profile_name.to_string(),
        form,
    };
    view
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Up arrow re-enters the name field in edit mode
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn up_arrow_re_enters_name_field_in_edit_mode() {
    // @step Given the edit profile form is open for the profile "work-vllm"
    let mut view = edit_view("work-vllm");

    // @step When I press the Up arrow key
    view.handle_key(key(KeyCode::Up));

    // @step Then the name field becomes editable
    assert!(form_of(&view).is_editing_name);

    // @step Given the cursor is focused on the Base URL field
    // (the Up above moved focus back onto the editable name from field 0)

    // @step When I type the character "2"
    view.handle_key(key(KeyCode::Char('2')));

    // @step Then the profile name becomes "work-vllm2"
    assert_eq!(form_of(&view).name, "work-vllm2");
}

// ─────────────────────────────────────────────────────────────────────────
// Save-emit: an edit-mode save carries the ORIGINAL name as old_profile_name
// so the persistence layer can detect + apply a rename. This is the view-side
// half of "Renaming a profile writes the new name and removes the old name".
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn edit_mode_save_emits_original_name_as_old_profile_name() {
    // @step Given the edit profile form is open for the profile "work-vllm"
    let mut view = edit_view("work-vllm");

    // @step When I press the Up arrow key
    view.handle_key(key(KeyCode::Up));
    // @step When the profile is renamed to "work-vllm-2" and saved
    for c in "-2".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    let ev = view.handle_key(key(KeyCode::Enter));

    // @step Then the config has a profile named "work-vllm-2"
    let (provider_id, new_name, old_name) = save_action(ev).expect("SaveProfile emitted");
    assert_eq!(provider_id, "openai");
    assert_eq!(new_name, "work-vllm-2");
    // @step Then the config no longer has a profile named "work-vllm"
    assert_eq!(old_name.as_deref(), Some("work-vllm"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: An empty name cannot be saved
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_name_cannot_be_saved() {
    // @step Given the edit profile form is open for the profile "work-vllm"
    let mut view = edit_view("work-vllm");

    // @step When the name is cleared and the form is submitted
    view.handle_key(key(KeyCode::Up));
    for _ in 0.."work-vllm".len() {
        view.handle_key(key(KeyCode::Backspace));
    }
    let ev = view.handle_key(key(KeyCode::Enter));

    // @step Then no save is performed
    assert!(save_action(ev).is_none());
    // @step Then the form stays open
    assert!(matches!(
        view.mode,
        ProviderSettingsMode::EditProfile { .. }
    ));
}
