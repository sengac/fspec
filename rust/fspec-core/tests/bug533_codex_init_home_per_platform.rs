//! BUG-533 — Codex init writes its prompt to the platform's home directory.
//!
//! Feature: spec/features/codex-init-writes-its-prompt-to-the-platform-home-directory.feature
//!
//! The platform and its environment are injected (`HomeEnv::from_vars`), so
//! every scenario runs the same on any OS and never reads the machine's real
//! home or process environment.

use std::ffi::OsString;
use std::path::Path;

use codelet_fspec_core::commands::init::{run_with_home, HomeEnv, HomePlatform};
use codelet_test_helpers::{OrFailErr, TestResult};
use serde_json::json;
use tempfile::TempDir;

/// The injected environment: `vars` are the only variables that exist.
fn env(platform: HomePlatform, vars: &[(&str, &Path)]) -> HomeEnv {
    let vars: Vec<(String, OsString)> = vars
        .iter()
        .map(|(name, value)| ((*name).to_string(), value.as_os_str().to_owned()))
        .collect();
    HomeEnv::from_vars(platform, |name| {
        vars.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    })
}

fn init(
    project: &Path,
    agent: &str,
    home: &HomeEnv,
) -> Result<String, codelet_fspec_core::FspecCoreError> {
    run_with_home(&json!({ "agent": [agent] }).to_string(), project, home)
}

#[test]
fn on_windows_the_codex_prompt_goes_to_the_user_profile() -> TestResult {
    // @step Given the platform is Windows with USERPROFILE set to a home directory and HOME unset
    let profile = TempDir::new()?;
    let home = env(HomePlatform::Windows, &[("USERPROFILE", profile.path())]);
    let project = TempDir::new()?;
    // @step When fspec is initialised for "codex" in an empty project
    init(project.path(), "codex", &home)?;
    // @step Then the home directory holds .codex/prompts/fspec.md
    let prompt = std::fs::read_to_string(profile.path().join(".codex/prompts/fspec.md"))?;
    assert!(prompt.contains("fspec bootstrap"), "{prompt}");
    // @step And the project has no .codex directory
    assert!(!project.path().join(".codex").exists());
    Ok(())
}

#[test]
fn on_unix_the_codex_prompt_goes_to_home() -> TestResult {
    // @step Given the platform is Unix with HOME set to a home directory
    let dir = TempDir::new()?;
    let home = env(HomePlatform::Unix, &[("HOME", dir.path())]);
    let project = TempDir::new()?;
    // @step When fspec is initialised for "codex-cli" in an empty project
    init(project.path(), "codex-cli", &home)?;
    // @step Then the home directory holds .codex/prompts/fspec.md
    let prompt = std::fs::read_to_string(dir.path().join(".codex/prompts/fspec.md"))?;
    assert!(prompt.contains("fspec bootstrap"), "{prompt}");
    // @step And the project has no .codex directory
    assert!(!project.path().join(".codex").exists());
    Ok(())
}

/// One example row of the refusal outline.
fn refuses_without_the_home_variable(
    platform: HomePlatform,
    home_variable: &str,
    other_variable: &str,
) -> TestResult {
    // @step Given the platform is <platform> with <home variable> unset and <other variable> set to a home directory
    let other = TempDir::new()?;
    let home = env(platform, &[(other_variable, other.path())]);
    let project = TempDir::new()?;
    // @step When fspec is initialised for "codex" in an empty project
    let error = init(project.path(), "codex", &home).or_fail_err("init refuses")?;
    // @step Then init fails saying the home directory cannot be found because <home variable> is not set
    let message = error.to_string();
    assert!(
        message.contains("home directory")
            && message.contains(&format!("{home_variable} is not set")),
        "{message}"
    );
    // @step And the project holds neither spec/AGENTS.md nor a .codex directory
    assert!(!project.path().join("spec/AGENTS.md").exists());
    assert!(!project.path().join(".codex").exists());
    // @step And the other directory holds no .codex directory
    assert!(!other.path().join(".codex").exists());
    Ok(())
}

#[test]
fn codex_init_refuses_when_the_platforms_home_variable_is_not_set() -> TestResult {
    refuses_without_the_home_variable(HomePlatform::Windows, "USERPROFILE", "HOME")?;
    refuses_without_the_home_variable(HomePlatform::Unix, "HOME", "USERPROFILE")?;
    Ok(())
}

#[test]
fn the_agent_doc_reads_the_user_config_from_the_windows_profile() -> TestResult {
    // @step Given the platform is Windows with USERPROFILE set to a home directory whose .fspec/fspec-config.json sets the test command "npm test"
    let profile = TempDir::new()?;
    std::fs::create_dir_all(profile.path().join(".fspec"))?;
    std::fs::write(
        profile.path().join(".fspec/fspec-config.json"),
        json!({"tools": {"test": {"command": "npm test"}}}).to_string(),
    )?;
    let home = env(HomePlatform::Windows, &[("USERPROFILE", profile.path())]);
    let project = TempDir::new()?;
    // @step When fspec is initialised for "claude" in an empty project
    init(project.path(), "claude", &home)?;
    // @step Then the project's spec/AGENTS.md has "npm test" in place of <test-command>
    let doc = std::fs::read_to_string(project.path().join("spec/AGENTS.md"))?;
    assert!(doc.contains("npm test"), "the doc has no npm test");
    assert!(!doc.contains("<test-command>"));
    Ok(())
}
