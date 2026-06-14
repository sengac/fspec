#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/configure-tools-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `configure-tools`
// (RPC-208). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Supervisor rulings honoured (orchestration-state.md):
//   - 2-arg run(args_json, project_root)
//   - D3 (installAgentFiles/init template regen) deferred — NOT tested.
//   - D4 (reconfigure message NOT wrapped in <system-reminder>) is reproduced
//     bug-for-bug; the reconfigure scenario only asserts the RECONFIGURE TOOLS
//     substring and that no config file is written (the unwrapped-output detail
//     is asserted at the CLI surface alongside the // TODO(parity-bug ...) marker).

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "configure-tools".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn config_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join("spec/fspec-config.json")
}

fn read_config(project_root: &Path) -> Value {
    let raw = fs::read_to_string(config_path(project_root)).expect("read fspec-config.json");
    serde_json::from_str(&raw).expect("parse fspec-config.json")
}

fn write_config(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("fspec-config.json"),
        serde_json::to_string_pretty(value).expect("ser config"),
    )
    .expect("write fspec-config.json");
}

// ---------- scenarios ----------

#[test]
fn setting_only_test_command_writes_it_under_tools_test_command() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!config_path(tmp.path()).exists());

    // @step When I dispatch configure-tools with testCommand='cargo test'
    let result = dispatch_command(req(tmp.path(), json!({"testCommand": "cargo test"})));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then spec/fspec-config.json exists on disk
    assert!(config_path(tmp.path()).exists());

    // @step And spec/fspec-config.json shows tools.test.command='cargo test'
    let v = read_config(tmp.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("cargo test"));

    // @step And spec/fspec-config.json shows agent='claude'
    assert_eq!(v["agent"].as_str(), Some("claude"));
}

#[test]
fn setting_both_test_command_and_quality_commands_persists_both() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch configure-tools with testCommand='npm test' and qualityCommands=['eslint .','prettier --check .']
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "testCommand": "npm test",
            "qualityCommands": ["eslint .", "prettier --check ."]
        }),
    ));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then spec/fspec-config.json shows tools.test.command='npm test'
    let v = read_config(tmp.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("npm test"));

    // @step And spec/fspec-config.json shows tools.qualityCheck.commands=['eslint .','prettier --check .']
    let cmds = v["tools"]["qualityCheck"]["commands"]
        .as_array()
        .expect("commands array");
    let cmds: Vec<&str> = cmds.iter().filter_map(Value::as_str).collect();
    assert_eq!(cmds, vec!["eslint .", "prettier --check ."]);
}

#[test]
fn reconfigure_flag_short_circuits_without_writing_config() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!config_path(tmp.path()).exists());

    // @step When I dispatch configure-tools with reconfigure=true
    let result = dispatch_command(req(tmp.path(), json!({"reconfigure": true})));

    // @step Then the dispatcher result contains the substring 'RECONFIGURE TOOLS'
    let haystack = if result.success {
        result.data
    } else {
        result.error.unwrap_or_default()
    };
    assert!(
        haystack.contains("RECONFIGURE TOOLS"),
        "expected RECONFIGURE TOOLS reminder; got: {haystack}"
    );

    // @step And spec/fspec-config.json does not exist on disk
    assert!(
        !config_path(tmp.path()).exists(),
        "reconfigure must NOT write the config file"
    );
}

#[test]
fn second_run_preserves_previously_stored_quality_commands() {
    // @step Given a project root tempdir whose spec/fspec-config.json already has tools.qualityCheck.commands=['eslint .']
    let tmp = TempDir::new().expect("tempdir");
    write_config(
        tmp.path(),
        &json!({
            "agent": "claude",
            "tools": {
                "qualityCheck": {"commands": ["eslint ."]}
            }
        }),
    );

    // @step When I dispatch configure-tools with testCommand='npm test'
    let result = dispatch_command(req(tmp.path(), json!({"testCommand": "npm test"})));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then spec/fspec-config.json shows tools.test.command='npm test'
    let v = read_config(tmp.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("npm test"));

    // @step And spec/fspec-config.json still shows tools.qualityCheck.commands=['eslint .']
    let cmds = v["tools"]["qualityCheck"]["commands"]
        .as_array()
        .expect("commands array preserved");
    let cmds: Vec<&str> = cmds.iter().filter_map(Value::as_str).collect();
    assert_eq!(cmds, vec!["eslint ."]);
}
