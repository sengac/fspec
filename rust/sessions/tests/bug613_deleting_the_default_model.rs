//! BUG-613: deleting the custom model that is fspec's default model, or the
//! profile that holds it, clears the default; session creation then names
//! how to choose one.
//!
//! Feature: spec/features/deleting-the-default-model-clears-the-default.feature
//!
//! A real SessionManager with FSPEC_USER_DIR (fspec-config.json, including
//! tui.lastUsedModel) and the data directory (default-model.json) in a temp
//! dir; serialized because both are process-global.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_sessions::SessionManager;
use serde_json::{json, Value};
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DEFAULT: &str = "openai:local/m1";

/// An openai profile "local" with custom models m1 and m2, and a manager
/// whose default model is `default`.
fn manager_with_default(
    dir: &Path,
    default: &str,
) -> Result<Arc<SessionManager>, Box<dyn std::error::Error>> {
    let config = json!({"providers": {"openai": {"profiles": {"local": {
        "baseUrl": "http://localhost:8000",
        "customModels": [{"id": "m1"}, {"id": "m2"}]
    }}}}});
    std::fs::write(
        dir.join("fspec-config.json"),
        serde_json::to_string_pretty(&config)?,
    )?;
    std::env::set_var("FSPEC_USER_DIR", dir);
    codelet_common::set_data_directory(dir.to_path_buf())?;
    let manager = SessionManager::new();
    manager.set_default_model(default)?;
    Ok(manager)
}

fn config(dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join("fspec-config.json"),
    )?)?)
}

fn persisted_default(dir: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(codelet_sessions::default_model_persistence::load_default_model_with_dir(dir)?)
}

#[test]
#[serial]
fn deleting_the_default_custom_model_clears_the_default() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given the default model is "openai:local/m1", a custom model of profile "local"
    let manager = manager_with_default(tmp.path(), DEFAULT)?;
    let handle: Arc<dyn SessionManagerHandle> = manager;

    // @step When the custom model m1 of openai profile "local" is deleted
    handle.delete_custom_model("openai", "local", "m1")?;

    // @step Then the model is gone from the profile
    let after = config(tmp.path())?;
    assert_eq!(
        after["providers"]["openai"]["profiles"]["local"]["customModels"],
        json!([{"id": "m2"}])
    );
    // @step And get_default_model returns no model
    assert_eq!(handle.get_default_model(), None);
    // @step And default-model.json and tui.lastUsedModel no longer name "openai:local/m1"
    assert_eq!(persisted_default(tmp.path())?, None);
    assert_ne!(after["tui"]["lastUsedModel"], json!(DEFAULT));
    Ok(())
}

#[test]
#[serial]
fn deleting_another_custom_model_leaves_the_default() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given the default model is "anthropic/claude-opus-4-5"
    let manager = manager_with_default(tmp.path(), "anthropic/claude-opus-4-5")?;
    let handle: Arc<dyn SessionManagerHandle> = manager;

    // @step When the custom model m1 of openai profile "local" is deleted
    handle.delete_custom_model("openai", "local", "m1")?;

    // @step Then get_default_model still returns "anthropic/claude-opus-4-5"
    assert_eq!(
        handle.get_default_model().as_deref(),
        Some("anthropic/claude-opus-4-5")
    );
    assert_eq!(
        persisted_default(tmp.path())?.as_deref(),
        Some("anthropic/claude-opus-4-5")
    );
    Ok(())
}

#[test]
#[serial]
fn deleting_the_profile_that_holds_the_default_clears_the_default() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given the default model is "openai:local/m1"
    let manager = manager_with_default(tmp.path(), DEFAULT)?;
    let handle: Arc<dyn SessionManagerHandle> = manager;

    // @step When the openai profile "local" is deleted
    handle.delete_profile("openai", "local")?;

    // @step Then get_default_model returns no model
    assert_eq!(handle.get_default_model(), None);
    // @step And a session manager started afterwards has no default model
    let restarted = SessionManager::new();
    restarted.restore_persisted_default_model()?;
    assert_eq!(restarted.get_default_model(), None);
    assert_ne!(config(tmp.path())?["tui"]["lastUsedModel"], json!(DEFAULT));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn creating_a_session_without_a_default_model_names_how_to_choose_one() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // @step Given the default model was cleared by deleting it
    let manager = manager_with_default(tmp.path(), DEFAULT)?;
    let handle: Arc<dyn SessionManagerHandle> = manager;
    handle.delete_custom_model("openai", "local", "m1")?;

    // @step When a session is created
    let created = handle.create_session_checked(None);

    // @step Then it fails with an error naming /model and set_default_model
    let error = created
        .err()
        .ok_or("a session was created without a default model")?;
    assert!(
        error.contains("/model") && error.contains("set_default_model"),
        "{error}"
    );
    Ok(())
}
