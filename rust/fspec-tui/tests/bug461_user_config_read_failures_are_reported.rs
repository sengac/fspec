//! BUG-461 — user config read failures are reported and never overwritten.
//!
//! Feature: spec/features/user-config-read-failures-are-reported.feature

use codelet_fspec_tui::views::provider_settings::profiles_config::load_openai_profiles_from;
use codelet_sessions::last_used_model_persistence::{
    load_persisted_model_string_from, save_persisted_model_string_to,
};
use codelet_sessions::mux_config_persistence::save_mux_config_with_dirs;
use codelet_sessions::profile_persistence::{save_profile_at, ProfileDef};
use codelet_test_helpers::TestResult;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serial_test::serial;

mod common;
use common::harness::AppTestHarness;

const MALFORMED: &str = "{not json";

/// A user directory whose fspec-config.json contains `{not json`.
fn malformed_user_dir() -> Result<tempfile::TempDir, std::io::Error> {
    let user = tempfile::tempdir()?;
    std::fs::write(user.path().join("fspec-config.json"), MALFORMED)?;
    Ok(user)
}

fn assert_unchanged(user: &tempfile::TempDir) -> TestResult {
    let content = std::fs::read_to_string(user.path().join("fspec-config.json"))?;
    assert_eq!(content, MALFORMED);
    Ok(())
}

fn assert_names_config(error: &str) {
    assert!(error.contains("fspec-config.json"), "{error}");
}

#[test]
fn saving_a_profile_refuses_a_malformed_user_config_and_leaves_it_unchanged() -> TestResult {
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    // @step When an openai profile is saved
    let definition = ProfileDef {
        base_url: "http://127.0.0.1:1/v1".to_string(),
        api_key: String::new(),
        context_window: None,
        max_output_tokens: None,
        compaction_threshold: None,
        streaming: None,
        auto_continue: None,
        preserve_thinking: None,
        max_images: None,
        loop_detection_enabled: None,
        loop_detection_window: None,
        loop_detection_max_repeats: None,
        loop_detection_max_retries: None,
    };
    let saved = save_profile_at(
        &user.path().join("fspec-config.json"),
        "openai",
        "local",
        &definition,
    );
    // @step Then the save fails naming fspec-config.json
    assert_names_config(&saved.err().ok_or("the profile was saved")?.to_string());
    // @step And the user fspec-config.json still contains "{not json"
    assert_unchanged(&user)
}

#[test]
fn saving_the_last_used_model_refuses_a_malformed_user_config_and_leaves_it_unchanged() -> TestResult
{
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    // @step When the last-used model is saved
    let saved = save_persisted_model_string_to(user.path(), "anthropic/claude-sonnet-4");
    // @step Then the save fails naming fspec-config.json
    assert_names_config(&saved.err().ok_or("the model was saved")?);
    // @step And the user fspec-config.json still contains "{not json"
    assert_unchanged(&user)
}

#[test]
fn saving_the_mux_config_refuses_a_malformed_user_config_and_leaves_it_unchanged() -> TestResult {
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    let project = tempfile::tempdir()?;
    // @step When the mux config is saved to the user scope
    let saved = save_mux_config_with_dirs(
        user.path(),
        project.path(),
        &serde_json::json!({"layout": "board:23, agent:27", "focused_pane": 1, "enabled": true}),
    );
    // @step Then the save fails naming fspec-config.json
    assert_names_config(&saved.err().ok_or("the mux config was saved")?);
    // @step And the user fspec-config.json still contains "{not json"
    assert_unchanged(&user)
}

#[test]
fn loading_the_last_used_model_from_a_malformed_user_config_is_an_error() -> TestResult {
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    // @step When the last-used model is loaded
    let loaded = load_persisted_model_string_from(user.path());
    // @step Then loading fails naming fspec-config.json
    assert_names_config(&loaded.err().ok_or("a model was loaded")?);
    Ok(())
}

#[test]
fn the_provider_settings_profile_loader_reports_a_malformed_config() -> TestResult {
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    let project = tempfile::tempdir()?;
    // @step When the openai profiles are loaded for provider settings
    let loaded = load_openai_profiles_from(user.path(), project.path());
    // @step Then loading fails naming fspec-config.json
    assert_names_config(&loaded.err().ok_or("profiles were loaded")?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn startup_shows_a_malformed_user_config_instead_of_choosing_a_model() -> TestResult {
    // @step Given the user fspec-config.json contains "{not json"
    let user = malformed_user_dir()?;
    std::env::set_var("FSPEC_USER_DIR", user.path());
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    // @step When the TUI starts
    let started = harness.app.bootstrap().await;
    std::env::remove_var("FSPEC_USER_DIR");
    started?;
    harness.drain_pending().await?;
    // @step Then an error dialog shows "The last used model could not be read"
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    let screen = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        screen.contains("The last used model could not be read"),
        "{screen}"
    );
    // @step And no startup model is set
    assert_eq!(harness.mock.set_default_model_calls(), 0);
    Ok(())
}
