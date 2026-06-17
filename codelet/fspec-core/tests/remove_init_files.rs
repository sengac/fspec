#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-init-files-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of the
// `remove-init-files` command (RPC-276). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the `remove-init-files` command is still a stub returning
// FspecCoreError::NotYetPorted, so every assertion below FAILS until Phase C
// wires the real implementation.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-init-files".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_config(root: &Path, agent: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("fspec-config.json"),
        json!({ "agent": agent }).to_string(),
    )
    .expect("write fspec-config.json");
}

/// Create an empty file at `root/rel`, making parent directories as needed.
fn touch(root: &Path, rel: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&path, "").expect("touch file");
}

fn files_removed(data: &str) -> Vec<String> {
    let parsed: Value = serde_json::from_str(data).expect("data must be JSON");
    parsed
        .get("filesRemoved")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .expect("root object must have filesRemoved array")
}

// ───────── scenarios ─────────

#[test]
fn removes_agent_files_and_config_when_the_config_names_claude() {
    // Scenario: Removes agent files and config when the config names claude

    // @step Given a workspace with spec/fspec-config.json containing agent='claude'
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "claude");

    // @step And the files spec/CLAUDE.md and .claude/commands/fspec.md exist
    touch(tmp.path(), "spec/CLAUDE.md");
    touch(tmp.path(), ".claude/commands/fspec.md");

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let removed = files_removed(&result.data);

    // @step And the returned JSON filesRemoved includes 'spec/CLAUDE.md'
    assert!(
        removed.iter().any(|f| f == "spec/CLAUDE.md"),
        "filesRemoved must include spec/CLAUDE.md; got {removed:?}"
    );

    // @step And the returned JSON filesRemoved includes '.claude/commands/fspec.md'
    assert!(
        removed.iter().any(|f| f == ".claude/commands/fspec.md"),
        "filesRemoved must include .claude/commands/fspec.md; got {removed:?}"
    );

    // @step And the returned JSON filesRemoved includes 'spec/fspec-config.json'
    assert!(
        removed.iter().any(|f| f == "spec/fspec-config.json"),
        "filesRemoved must include spec/fspec-config.json; got {removed:?}"
    );

    // @step And spec/CLAUDE.md no longer exists
    assert!(
        !tmp.path().join("spec/CLAUDE.md").exists(),
        "spec/CLAUDE.md must be removed"
    );

    // @step And spec/fspec-config.json no longer exists
    assert!(
        !tmp.path().join("spec/fspec-config.json").exists(),
        "spec/fspec-config.json must be removed"
    );
}

#[test]
fn detects_a_toml_agent_by_its_detection_directory_when_no_config_is_present() {
    // Scenario: Detects a toml agent by its detection directory when no config is present

    // @step Given a workspace with no spec/fspec-config.json but a .gemini/ directory exists
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".gemini")).expect("mkdir .gemini");

    // @step And the files spec/GEMINI.md and .gemini/commands/fspec.toml exist
    touch(tmp.path(), "spec/GEMINI.md");
    touch(tmp.path(), ".gemini/commands/fspec.toml");

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let removed = files_removed(&result.data);

    // @step And the returned JSON filesRemoved includes 'spec/GEMINI.md'
    assert!(
        removed.iter().any(|f| f == "spec/GEMINI.md"),
        "filesRemoved must include spec/GEMINI.md; got {removed:?}"
    );

    // @step And the returned JSON filesRemoved includes '.gemini/commands/fspec.toml'
    assert!(
        removed.iter().any(|f| f == ".gemini/commands/fspec.toml"),
        "filesRemoved must include .gemini/commands/fspec.toml; got {removed:?}"
    );
}

#[test]
fn keep_config_true_preserves_spec_fspec_config_json() {
    // Scenario: keepConfig=true preserves spec/fspec-config.json

    // @step Given a workspace with spec/fspec-config.json containing agent='claude'
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "claude");

    // @step And the files spec/CLAUDE.md and .claude/commands/fspec.md exist
    touch(tmp.path(), "spec/CLAUDE.md");
    touch(tmp.path(), ".claude/commands/fspec.md");

    // @step When I dispatch remove-init-files with keepConfig=true
    let result = dispatch_command(req(tmp.path(), json!({ "keepConfig": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let removed = files_removed(&result.data);

    // @step And the returned JSON filesRemoved includes 'spec/CLAUDE.md'
    assert!(
        removed.iter().any(|f| f == "spec/CLAUDE.md"),
        "filesRemoved must include spec/CLAUDE.md; got {removed:?}"
    );

    // @step And the returned JSON filesRemoved does NOT include 'spec/fspec-config.json'
    assert!(
        !removed.iter().any(|f| f == "spec/fspec-config.json"),
        "filesRemoved must NOT include spec/fspec-config.json; got {removed:?}"
    );

    // @step And spec/fspec-config.json still exists
    assert!(
        tmp.path().join("spec/fspec-config.json").exists(),
        "spec/fspec-config.json must be preserved"
    );
}

#[test]
fn errors_when_no_agent_installation_is_detected() {
    // Scenario: Errors when no agent installation is detected

    // @step Given a workspace with no spec/fspec-config.json and no agent detection directories
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message containing 'No fspec agent installation detected. Nothing to remove.'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("No fspec agent installation detected. Nothing to remove."),
        "error message missing substring: {msg}"
    );
}

#[test]
fn force_removal_is_idempotent_when_an_agent_file_is_already_absent() {
    // Scenario: Force removal is idempotent when an agent file is already absent

    // @step Given a workspace with spec/fspec-config.json containing agent='claude'
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "claude");

    // @step And spec/CLAUDE.md does NOT exist but .claude/commands/fspec.md exists
    touch(tmp.path(), ".claude/commands/fspec.md");
    assert!(
        !tmp.path().join("spec/CLAUDE.md").exists(),
        "precondition: spec/CLAUDE.md must not exist"
    );

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON filesRemoved includes 'spec/CLAUDE.md'
    let removed = files_removed(&result.data);
    assert!(
        removed.iter().any(|f| f == "spec/CLAUDE.md"),
        "filesRemoved must include spec/CLAUDE.md (idempotent force removal); got {removed:?}"
    );
}

#[test]
fn errors_when_the_config_names_an_unknown_agent() {
    // Scenario: Errors when the config names an unknown agent

    // @step Given a workspace with spec/fspec-config.json containing agent='not-a-real-agent'
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "not-a-real-agent");

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message containing 'Unknown agent: not-a-real-agent'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Unknown agent: not-a-real-agent"),
        "error message missing substring: {msg}"
    );
}

#[test]
fn does_not_touch_project_files() {
    // Scenario: Does not touch project files

    // @step Given a workspace with spec/fspec-config.json containing agent='claude'
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), "claude");

    // @step And spec/work-units.json and a spec/features/ directory exist
    touch(tmp.path(), "spec/work-units.json");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir spec/features");

    // @step When I dispatch remove-init-files with no keepConfig
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json still exists
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must NOT be removed"
    );

    // @step And the spec/features/ directory still exists
    assert!(
        tmp.path().join("spec/features").is_dir(),
        "spec/features/ must NOT be removed"
    );
}
