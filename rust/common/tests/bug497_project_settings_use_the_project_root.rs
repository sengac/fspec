//! BUG-497 — project settings use the resolved project root.
//!
//! Feature: spec/features/project-settings-and-the-core-board-use-the-resolved-project-root.feature

use std::path::PathBuf;

use codelet_common::fspec_config::{load_config_with_dirs, update_config_with_dirs, ConfigScope};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A project root (with a boundary marker and spec/) holding `rust/sub`, and
/// a separate user data directory.
fn project() -> Result<(tempfile::TempDir, PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("Cargo.toml"), "[workspace]\n")?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    let sub = root.path().join("rust/sub");
    std::fs::create_dir_all(&sub)?;
    let data = root.path().join("data");
    std::fs::create_dir_all(&data)?;
    Ok((root, sub, data))
}

#[test]
fn saving_project_settings_from_a_subdirectory_writes_the_projects_spec_folder() -> TestResult {
    // @step Given a project with a spec folder and a subdirectory without one
    let (root, sub, data) = project()?;
    // @step When project settings are saved from the subdirectory
    update_config_with_dirs(ConfigScope::Project, &data, &sub, |root| {
        *root = json!({"agent": "claude"});
        Ok::<_, String>(())
    })?;
    // @step Then the project's spec/fspec-config.json holds the settings
    let saved = std::fs::read_to_string(root.path().join("spec/fspec-config.json"))?;
    assert!(saved.contains("\"claude\""), "{saved}");
    // @step And the subdirectory has no spec folder
    assert!(!sub.join("spec").exists());
    Ok(())
}

#[test]
fn loading_project_settings_from_a_subdirectory_reads_the_projects_spec_folder() -> TestResult {
    // @step Given a project whose spec/fspec-config.json sets a value
    let (root, sub, data) = project()?;
    std::fs::write(
        root.path().join("spec/fspec-config.json"),
        r#"{"agent": "codex"}"#,
    )?;
    // @step When the settings are loaded from a subdirectory
    let loaded = load_config_with_dirs(&data, &sub)?;
    // @step Then the loaded settings carry that value
    assert_eq!(loaded["agent"], "codex", "{loaded}");
    Ok(())
}
