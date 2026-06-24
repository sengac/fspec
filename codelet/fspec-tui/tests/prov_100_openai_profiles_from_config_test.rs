//! PROV-100 — OpenAI custom profiles loaded from fspec-config.json.
//!
//! Feature: spec/features/openai-profiles-from-config.feature
//!
//! Exercises the pure, path-injectable loader
//! `profiles_config::load_openai_profiles_from(user_dir, project_root)`
//! plus the loader→projection→nav-build integration. All filesystem
//! state is created in per-test temp dirs (`tempfile::tempdir`): no real
//! `$HOME`, no env mutation, no network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use codelet_fspec_tui::views::provider_settings::nav_item::{
    build_nav_items, NavItemKind, ProviderDisplayInfo,
};
use codelet_fspec_tui::views::provider_settings::profiles_config::load_openai_profiles_from;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

/// Write `<dir>/fspec-config.json` with the given raw contents, creating
/// `dir` if necessary.
fn write_user_config(dir: &Path, contents: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("fspec-config.json"), contents).unwrap();
}

/// Write `<project_root>/spec/fspec-config.json` with the given raw
/// contents, creating the `spec` dir if necessary.
fn write_project_config(project_root: &Path, contents: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).unwrap();
    fs::write(spec.join("fspec-config.json"), contents).unwrap();
}

/// A minimal fspec-config.json string with one openai profile carrying a
/// baseUrl.
fn cfg_one_profile(name: &str, base_url: &str) -> String {
    format!(
        r#"{{ "providers": {{ "openai": {{ "profiles": {{ "{name}": {{ "baseUrl": "{base_url}" }} }} }} }} }}"#
    )
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: User config profile is loaded and formatted as name → baseUrl
// ────────────────────────────────────────────────────────────────────────

#[test]
fn user_profile_formatted_as_name_arrow_base_url() {
    // @step Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://api.fireworks.ai/inference"
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        &cfg_one_profile("fireworks", "https://api.fireworks.ai/inference"),
    );

    // @step And an empty project config directory
    // (project tempdir intentionally left without spec/fspec-config.json)

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is the single display string "fireworks → https://api.fireworks.ai/inference"
    assert_eq!(
        profiles,
        vec!["fireworks → https://api.fireworks.ai/inference"]
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Project profile overrides user profile by name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn project_profile_overrides_user_by_name() {
    // @step Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://user.example/v1"
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        &cfg_one_profile("fireworks", "https://user.example/v1"),
    );

    // @step And a project config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://project.example/v1"
    write_project_config(
        project.path(),
        &cfg_one_profile("fireworks", "https://project.example/v1"),
    );

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is the single display string "fireworks → https://project.example/v1"
    assert_eq!(profiles, vec!["fireworks → https://project.example/v1"]);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: User and project profiles are merged and sorted by name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn user_and_project_profiles_merged_and_sorted() {
    // @step Given a user config fspec-config.json with an openai profile "fireworks" whose baseUrl is "https://api.fireworks.ai/inference"
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        &cfg_one_profile("fireworks", "https://api.fireworks.ai/inference"),
    );

    // @step And a project config fspec-config.json with an openai profile "together" whose baseUrl is "https://api.together.xyz/v1"
    write_project_config(
        project.path(),
        &cfg_one_profile("together", "https://api.together.xyz/v1"),
    );

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is the display strings "fireworks → https://api.fireworks.ai/inference" then "together → https://api.together.xyz/v1" in that order
    assert_eq!(
        profiles,
        vec![
            "fireworks → https://api.fireworks.ai/inference".to_string(),
            "together → https://api.together.xyz/v1".to_string(),
        ]
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Profile without a baseUrl renders as just the name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn profile_without_base_url_renders_just_name() {
    // @step Given a user config fspec-config.json with an openai profile "local" that has no baseUrl
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        r#"{ "providers": { "openai": { "profiles": { "local": { "apiKey": "x" } } } } }"#,
    );

    // @step And an empty project config directory

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is the single display string "local"
    assert_eq!(profiles, vec!["local"]);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Missing config files yield no profiles
// ────────────────────────────────────────────────────────────────────────

#[test]
fn missing_config_files_yield_no_profiles() {
    // @step Given a user config directory with no fspec-config.json
    let user = tempfile::tempdir().unwrap();

    // @step And an empty project config directory
    let project = tempfile::tempdir().unwrap();

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is an empty list
    assert!(profiles.is_empty());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Malformed JSON yields no profiles without panicking
// ────────────────────────────────────────────────────────────────────────

#[test]
fn malformed_json_yields_no_profiles_without_panic() {
    // @step Given a user config fspec-config.json whose contents are malformed JSON
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(user.path(), "{ this is not valid json :: ");

    // @step And an empty project config directory

    // @step When load_openai_profiles_from is called with the user and project directories
    let profiles = load_openai_profiles_from(user.path(), project.path());

    // @step Then the result is an empty list
    assert!(profiles.is_empty());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: A loaded profile renders as a Profile row above Add Profile
// ────────────────────────────────────────────────────────────────────────

#[test]
fn loaded_profile_renders_above_add_profile() {
    // @step Given an OpenAI ProviderDisplayInfo whose profiles slice is the loader output "fireworks → https://api.fireworks.ai/inference"
    let user = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    write_user_config(
        user.path(),
        &cfg_one_profile("fireworks", "https://api.fireworks.ai/inference"),
    );
    let profiles = load_openai_profiles_from(user.path(), project.path());
    let openai = ProviderDisplayInfo {
        id: "openai".to_string(),
        name: "OpenAI API".to_string(),
        credential_type: "api_key".to_string(),
        requires_api_key: false,
        profiles,
        ..Default::default()
    };

    // @step And the openai provider is expanded
    let mut expanded = HashSet::new();
    expanded.insert("openai".to_string());

    // @step When the nav items are built
    let items = build_nav_items(&[openai], &expanded, "");

    // @step Then a Profile row "fireworks → https://api.fireworks.ai/inference" appears immediately above the Add Profile row
    let profile_idx = items
        .iter()
        .position(|it| {
            matches!(&it.kind, NavItemKind::Profile { profile_name }
                if profile_name == "fireworks → https://api.fireworks.ai/inference")
        })
        .expect("expected a Profile row for the fireworks profile");
    assert!(matches!(
        items[profile_idx + 1].kind,
        NavItemKind::AddProfile
    ));
}
