//! PROV-111 — profile nav routing, prefill, per-profile delete and the
//! full-config loader.
//!
//! Feature: spec/features/provider-settings-profile-crud.feature
//!
//! Offline view-layer + loader tests. The view tests drive Enter / `d`
//! through `ProviderSettingsView` against a rich RPC-103 nav tree (no App,
//! no backend, no tokio); the loader tests exercise the pure path-injectable
//! `load_openai_profile_configs_from` against per-test temp dirs (no real
//! `$HOME`, no env mutation, no network).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::profiles_config::load_openai_profile_configs_from;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

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

fn def(base_url: &str, api_key: &str) -> ProfileDefinition {
    ProfileDefinition {
        base_url: base_url.to_string(),
        api_key: api_key.to_string(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold_type: None,
        compaction_threshold_value: None,
        streaming: None,
    auto_continue: None,
    }
}

/// Build an OpenAI-only view, expanded, with the given profile display
/// strings and a per-profile config map. The cursor starts on the openai
/// provider row at index 0.
fn openai_view(
    profiles: &[&str],
    configs: HashMap<String, ProfileDefinition>,
) -> ProviderSettingsView {
    let creds = vec![pinfo("openai", "api_key", true, 5)];
    let profile_strings: Vec<String> = profiles.iter().map(|s| (*s).to_string()).collect();
    let display = project_display_infos(&creds, &profile_strings);
    let mut view = ProviderSettingsView::new();
    view.set_providers(creds);
    view.set_provider_display_infos(display);
    view.set_profile_configs(configs);
    // Expand openai so the profile + AddProfile child rows appear.
    view.handle_key(key(KeyCode::Enter));
    view
}

fn focused_kind(view: &ProviderSettingsView) -> NavItemKind {
    view.focused_nav_item()
        .expect("a nav item must be focused")
        .kind
        .clone()
}

fn write_user_config(dir: &Path, contents: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("fspec-config.json"), contents).unwrap();
}

fn write_project_config(project_root: &Path, contents: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).unwrap();
    fs::write(spec.join("fspec-config.json"), contents).unwrap();
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on a profile row opens the edit form prefilled from the
// stored config
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_profile_row_opens_edit_form_prefilled() {
    // @step Given the Provider Settings nav tree has an expanded "openai" provider with a stored profile "fireworks"
    // @step And the per-profile config map carries "fireworks" with baseUrl "https://api.fireworks.ai/inference/v1" and an apiKey
    let mut configs = HashMap::new();
    configs.insert(
        "fireworks".to_string(),
        def("https://api.fireworks.ai/inference/v1", "sk-stored"),
    );
    let mut view = openai_view(
        &["fireworks → https://api.fireworks.ai/inference/v1"],
        configs,
    );

    // @step And the cursor is on the "fireworks" profile row
    view.handle_key(key(KeyCode::Down));
    assert!(matches!(focused_kind(&view), NavItemKind::Profile { .. }));

    // @step When I press Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the view enters EditProfile mode for provider "openai" and profile "fireworks"
    match &view.mode {
        ProviderSettingsMode::EditProfile {
            provider_id,
            profile_name,
            form,
        } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "fireworks");
            // @step And the form base URL is prefilled with "https://api.fireworks.ai/inference/v1"
            assert_eq!(form.base_url, "https://api.fireworks.ai/inference/v1");
            // @step And the form api key is prefilled from the stored config
            assert_eq!(form.api_key, "sk-stored");
            // @step And the form name editing flag is false so "fireworks" is not editable
            assert!(!form.is_editing_name);
            assert_eq!(form.name, "fireworks");
        }
        other => panic!("expected EditProfile mode, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Enter on the Add Profile row opens a create form with defaults
// ────────────────────────────────────────────────────────────────────────

#[test]
fn enter_on_add_profile_opens_create_form_with_defaults() {
    // @step Given the Provider Settings nav tree has an expanded "openai" provider with a trailing Add Profile row
    let mut view = openai_view(&["fast → http://localhost:1"], HashMap::new());

    // @step And the cursor is on the Add Profile row
    view.handle_key(key(KeyCode::Down)); // profile "fast"
    view.handle_key(key(KeyCode::Down)); // AddProfile
    assert!(matches!(focused_kind(&view), NavItemKind::AddProfile));

    // @step When I press Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the view enters CreateProfile mode for provider "openai"
    match &view.mode {
        ProviderSettingsMode::CreateProfile { provider_id, form } => {
            assert_eq!(provider_id, "openai");
            // @step And the form base URL defaults to "http://localhost:8888"
            assert_eq!(form.base_url, "http://localhost:8888");
            // @step And the form api key is empty
            assert_eq!(form.api_key, "");
            // @step And the form name editing flag is true
            assert!(form.is_editing_name);
        }
        other => panic!("expected CreateProfile mode, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing d on a profile row opens a per-profile delete confirm
// that targets only that profile
// ────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_profile_row_opens_per_profile_delete_confirm() {
    // @step Given the Provider Settings nav tree has an expanded "openai" provider with profiles "fireworks" and "home"
    let mut view = openai_view(
        &[
            "fireworks → https://api.fireworks.ai/inference/v1",
            "home → http://localhost:8888",
        ],
        HashMap::new(),
    );

    // @step And the cursor is on the "home" profile row
    // (BTreeMap-sorted: fireworks at idx 1, home at idx 2)
    view.handle_key(key(KeyCode::Down));
    view.handle_key(key(KeyCode::Down));
    match focused_kind(&view) {
        NavItemKind::Profile { profile_name } => {
            assert_eq!(profile_name, "home → http://localhost:8888")
        }
        other => panic!("expected the 'home' profile row, got {other:?}"),
    }

    // @step When I press "d"
    view.handle_key(key(KeyCode::Char('d')));

    // @step Then a delete confirmation dialog is open
    assert!(view.delete_confirm.is_some());

    // @step When I accept the delete confirmation
    let out = view.handle_key(key(KeyCode::Enter));

    // @step Then a ConfirmDeleteProfile action is emitted for provider "openai" and profile "home"
    match out {
        ProviderSettingsEvent::Emit(Action::ConfirmDeleteProfile {
            provider_id,
            profile_name,
        }) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "home");
        }
        other => panic!("expected Emit(ConfirmDeleteProfile{{openai,home}}), got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing d on the Add Profile row has no delete action
// ────────────────────────────────────────────────────────────────────────

#[test]
fn d_on_add_profile_row_has_no_delete_action() {
    // @step Given the Provider Settings nav tree has an expanded "openai" provider with a trailing Add Profile row
    let mut view = openai_view(&["fast → http://localhost:1"], HashMap::new());

    // @step And the cursor is on the Add Profile row
    view.handle_key(key(KeyCode::Down)); // profile
    view.handle_key(key(KeyCode::Down)); // AddProfile
    assert!(matches!(focused_kind(&view), NavItemKind::AddProfile));

    // @step When I press "d"
    let out = view.handle_key(key(KeyCode::Char('d')));

    // @step Then no delete confirmation dialog is open
    assert!(view.delete_confirm.is_none());
    // @step And the key is consumed without emitting an action
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: The full-config loader returns parsed ProfileDefinitions keyed
// by name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn full_config_loader_returns_parsed_definitions_keyed_by_name() {
    // @step Given a user config fspec-config.json with an openai profile "fireworks" carrying a baseUrl, an apiKey and a contextWindow
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        r#"{ "providers": { "openai": { "profiles": { "fireworks": {
            "baseUrl": "https://api.fireworks.ai/inference/v1",
            "apiKey": "sk-fw",
            "contextWindow": 131072
        } } } } }"#,
    );

    // @step And an empty project config directory
    // (project tempdir intentionally left without spec/fspec-config.json)

    // @step When load_openai_profile_configs_from is called with the user and project directories
    let map = load_openai_profile_configs_from(user.path(), project.path());

    // @step Then the result maps "fireworks" to a ProfileDefinition whose base URL, api key and context window match the stored values
    let fw = map.get("fireworks").expect("fireworks present");
    assert_eq!(fw.base_url, "https://api.fireworks.ai/inference/v1");
    assert_eq!(fw.api_key, "sk-fw");
    assert_eq!(fw.context_window, Some(131072));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: The full-config loader merges with project overriding user by
// name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn full_config_loader_project_overrides_user_by_name() {
    // @step Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://user.example/v1"
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        r#"{ "providers": { "openai": { "profiles": { "fireworks": {
            "baseUrl": "https://user.example/v1", "apiKey": "sk-user"
        } } } } }"#,
    );

    // @step And a project config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://project.example/v1"
    write_project_config(
        project.path(),
        r#"{ "providers": { "openai": { "profiles": { "fireworks": {
            "baseUrl": "https://project.example/v1", "apiKey": "sk-proj"
        } } } } }"#,
    );

    // @step When load_openai_profile_configs_from is called with the user and project directories
    let map = load_openai_profile_configs_from(user.path(), project.path());

    // @step Then the result maps "fireworks" to a ProfileDefinition whose base URL is "https://project.example/v1"
    let fw = map.get("fireworks").expect("fireworks present");
    assert_eq!(fw.base_url, "https://project.example/v1");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Folding loaded credentials stores the per-profile config map for
// prefill
// ────────────────────────────────────────────────────────────────────────

#[test]
fn stored_config_map_is_queryable_by_profile_name() {
    // @step Given a Provider Settings view
    let mut view = ProviderSettingsView::new();

    // @step When a per-profile config map containing "fireworks" is stored on the view
    let mut configs = HashMap::new();
    configs.insert(
        "fireworks".to_string(),
        def("https://api.fireworks.ai/inference/v1", "sk-stored"),
    );
    view.set_profile_configs(configs);

    // @step Then profile_config_for("fireworks") returns the stored ProfileDefinition
    let got = view
        .profile_config_for("fireworks")
        .expect("fireworks present");
    assert_eq!(got.base_url, "https://api.fireworks.ai/inference/v1");
    assert_eq!(got.api_key, "sk-stored");

    // @step And profile_config_for("missing") returns nothing
    assert!(view.profile_config_for("missing").is_none());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: The profile row display string is split back to the bare name
// for lookup
// ────────────────────────────────────────────────────────────────────────

#[test]
fn profile_row_display_string_is_split_back_to_bare_name_for_lookup() {
    // @step Given a Provider Settings nav tree whose "openai" profile row label is "fireworks → https://api.fireworks.ai/inference/v1"
    // @step And the per-profile config map carries "fireworks" with an apiKey
    let mut configs = HashMap::new();
    configs.insert(
        "fireworks".to_string(),
        def("https://api.fireworks.ai/inference/v1", "sk-split"),
    );
    let mut view = openai_view(
        &["fireworks → https://api.fireworks.ai/inference/v1"],
        configs,
    );

    // @step And the cursor is on that profile row
    view.handle_key(key(KeyCode::Down));
    assert!(matches!(focused_kind(&view), NavItemKind::Profile { .. }));

    // @step When I press Enter
    view.handle_key(key(KeyCode::Enter));

    // @step Then the view enters EditProfile mode for profile "fireworks"
    // @step And the form is prefilled from the stored "fireworks" config
    match &view.mode {
        ProviderSettingsMode::EditProfile {
            profile_name, form, ..
        } => {
            assert_eq!(profile_name, "fireworks");
            assert_eq!(form.api_key, "sk-split");
            assert_eq!(form.base_url, "https://api.fireworks.ai/inference/v1");
        }
        other => panic!("expected EditProfile mode for fireworks, got {other:?}"),
    }
}
