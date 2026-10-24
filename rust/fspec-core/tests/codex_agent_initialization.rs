use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use serial_test::serial;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct HomeGuard(Option<std::ffi::OsString>);

impl HomeGuard {
    fn redirect(path: &Path) -> Self {
        let previous = Self(std::env::var_os("HOME"));
        std::env::set_var("HOME", path);
        previous
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(previous) => std::env::set_var("HOME", previous),
            None => std::env::remove_var("HOME"),
        }
    }
}

fn initialize(root: &Path, agent: &str) -> TestResult<Value> {
    let response = dispatch_command(DispatchRequest {
        command: "init".into(),
        args_json: json!({"agent": [agent]}).to_string(),
        project_root: root.into(),
    });
    assert!(response.success, "{response:?}");
    Ok(serde_json::from_str(&response.data)?)
}

fn assert_instructions(project: &Path, home: &Path) -> TestResult {
    let prompt = std::fs::read_to_string(home.join(".codex/prompts/fspec.md"))?;
    assert!(prompt.contains("fspec bootstrap"));
    let instructions = std::fs::read_to_string(project.join("spec/AGENTS.md"))?;
    for required in ["ACDD", "testing", "implementing", "validating", "fspec"] {
        assert!(instructions.contains(required), "missing {required}");
    }
    assert!(!instructions.contains("{{AGENT_NAME}}"));
    assert!(!instructions.contains("<system-reminder>"));
    Ok(())
}

/// Feature: spec/features/codex-agent-initialization.feature
#[test]
#[serial]
fn initialize_codex_with_usable_acdd_instructions() -> TestResult {
    // @step Given an empty project and an isolated HOME
    let project = tempfile::tempdir()?;
    let home = tempfile::tempdir()?;
    let _home = HomeGuard::redirect(home.path());
    // @step When I initialize fspec for codex
    let result = initialize(project.path(), "codex")?;
    // @step Then the home prompt and project ACDD instructions are installed
    assert_instructions(project.path(), home.path())?;
    // @step And the codex selection and activation command are reported correctly
    let config: Value = serde_json::from_str(&std::fs::read_to_string(
        project.path().join("spec/fspec-config.json"),
    )?)?;
    assert_eq!(config["agent"], "codex");
    assert_eq!(
        result["activationMessage"],
        "Run /prompts:fspec in Codex to activate"
    );
    assert_eq!(
        result["filesInstalled"],
        json!(["spec/AGENTS.md", "~/.codex/prompts/fspec.md"])
    );
    // @step And obsolete project-local Codex instructions are not created
    assert!(!project.path().join(".codex").exists());
    assert!(!project.path().join("spec/CODEX.md").exists());
    Ok(())
}

/// Feature: spec/features/codex-cli-agent-initialization.feature
#[test]
#[serial]
fn reinitialize_codex_cli_without_losing_project_work() -> TestResult {
    // @step Given an initialized Codex CLI project with work units and unrelated home prompts
    let project = tempfile::tempdir()?;
    let home = tempfile::tempdir()?;
    let _home = HomeGuard::redirect(home.path());
    initialize(project.path(), "codex-cli")?;
    let work_units = project.path().join("spec/work-units.json");
    let existing = br#"{"workUnits":{"EX-001":{"title":"Keep me","status":"backlog"}}}"#;
    std::fs::write(&work_units, existing)?;
    let unrelated = home.path().join(".codex/prompts/unrelated.md");
    std::fs::write(&unrelated, "user content")?;
    let config_path = project.path().join("spec/fspec-config.json");
    std::fs::write(&config_path, r#"{"agent":"codex-cli","custom":"keep"}"#)?;
    // @step When I initialize fspec for codex-cli again
    let result = initialize(project.path(), "codex-cli")?;
    // @step Then the home prompt and project ACDD instructions remain usable
    assert_instructions(project.path(), home.path())?;
    // @step And existing work units configuration and unrelated home prompts are preserved
    assert_eq!(std::fs::read(&work_units)?, existing);
    assert_eq!(std::fs::read_to_string(&unrelated)?, "user content");
    let config: Value = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
    assert_eq!(config["custom"], "keep");
    // @step And the Codex CLI activation command and persisted selection are correct
    assert_eq!(config["agent"], "codex-cli");
    assert_eq!(
        result["activationMessage"],
        "Run /prompts:fspec in Codex CLI to activate"
    );
    assert!(!project.path().join("spec/CODEX-CLI.md").exists());
    Ok(())
}
