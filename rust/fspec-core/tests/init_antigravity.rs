//! AGENT-020 — `fspec init` installs fspec for the Antigravity agent.
//!
//! Feature: spec/features/test-antigravity-support.feature
//!
//! The Antigravity registry entry (ANTIGRAVITY.md, `.antigravity/commands/`)
//! is installed through the same `init` dispatcher the CLI calls; the CLI
//! prints the `filesInstalled` list and the activation message it returns.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Dispatch `init` for `agents` against `root`; the parsed result data, or
/// the dispatcher's error.
fn init(root: &Path, agents: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "init".to_string(),
        args_json: json!({ "agent": agents }).to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("init {agents:?} failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

#[test]
fn initialize_fspec_with_antigravity_agent() -> TestResult {
    // @step Given I am in a new project directory
    let tmp = tempfile::tempdir()?;

    // @step When I dispatch init against that project directory with agent list ['antigravity']
    // @step Then the dispatcher returns success=true
    let data = init(tmp.path(), &["antigravity"])?;
    assert_eq!(data["success"], json!(true), "data: {data}");

    // @step And the filesInstalled array is 'spec/ANTIGRAVITY.md' and '.antigravity/commands/fspec.md'
    assert_eq!(
        data["filesInstalled"],
        json!(["spec/ANTIGRAVITY.md", ".antigravity/commands/fspec.md"])
    );

    // @step And spec/ANTIGRAVITY.md and .antigravity/commands/fspec.md exist in the project directory
    assert!(tmp.path().join("spec/ANTIGRAVITY.md").is_file());
    assert!(tmp.path().join(".antigravity/commands/fspec.md").is_file());

    // @step And the activation message is 'Add .antigravity/commands/ to your Antigravity configuration to activate'
    assert_eq!(
        data["activationMessage"],
        json!("Add .antigravity/commands/ to your Antigravity configuration to activate")
    );
    Ok(())
}

#[test]
fn the_project_config_records_the_antigravity_agent() -> TestResult {
    // @step Given I have initialized fspec with the "antigravity" agent
    let tmp = tempfile::tempdir()?;
    init(tmp.path(), &["antigravity"])?;

    // @step When I read spec/fspec-config.json
    let raw = std::fs::read_to_string(tmp.path().join("spec/fspec-config.json"))?;
    let config: Value = serde_json::from_str(&raw)?;

    // @step Then its agent field is 'antigravity'
    assert_eq!(config["agent"], json!("antigravity"), "config: {config}");
    Ok(())
}
